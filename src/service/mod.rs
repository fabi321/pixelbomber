mod converter;
mod distributor;
mod host;
mod merger;
mod painter;

use std::{
    sync::{
        mpsc::{sync_channel, SyncSender},
        Arc,
    },
    thread::{spawn, JoinHandle},
};

pub use host::Host;

use crate::{
    image_handler::{Command, ImageConfig},
    Client,
};

pub struct ServiceBuilder {
    host: Host,
    threads: usize,
    image_config: ImageConfig,
    converter_threads: usize,
    channel_limit: usize,
}

impl ServiceBuilder {
    /// Create a new ServiceBuilder
    pub fn new(host: Host) -> ServiceBuilder {
        ServiceBuilder {
            host,
            threads: 10,
            image_config: ImageConfig::default(),
            converter_threads: 1,
            channel_limit: 10,
        }
    }

    /// Create a new ServiceBuilder from a host string
    pub fn new_from_host_str(host: &str) -> ServiceBuilder {
        ServiceBuilder::new(Host::new(host, None).unwrap())
    }

    /// Set the number of painter threads for the service
    pub fn threads(mut self, threads: usize) -> ServiceBuilder {
        self.threads = threads;
        self
    }

    /// Set the image processing configuration
    pub fn image_config(mut self, image_config: ImageConfig) -> ServiceBuilder {
        self.image_config = image_config;
        self
    }

    /// Set the number of converter threads for the service
    pub fn converter_threads(mut self, converter_threads: usize) -> ServiceBuilder {
        self.converter_threads = converter_threads;
        self
    }

    /// Set the limit for communication channels
    pub fn channel_limit(mut self, channel_limit: usize) -> ServiceBuilder {
        self.channel_limit = channel_limit;
        self
    }

    pub fn build(self) -> Service {
        Service::new(
            self.host,
            self.threads,
            self.image_config,
            self.converter_threads,
            self.channel_limit,
        )
    }
}

pub struct Service {
    host: Host,
    threads: usize,
    image_config: ImageConfig,
    worker_client: Option<Client>,
    converter_threads: usize,
    channel_limit: usize,
    converter_input: Option<SyncSender<distributor::DistributorChange>>,
    painter_input: Option<SyncSender<Arc<Command>>>,
    join_handles: Vec<JoinHandle<()>>,
}

impl Service {
    pub fn new(
        host: Host,
        threads: usize,
        image_config: ImageConfig,
        converter_threads: usize,
        channel_limit: usize,
    ) -> Service {
        Service {
            host,
            threads,
            image_config,
            worker_client: None,
            converter_threads,
            channel_limit,
            converter_input: None,
            painter_input: None,
            join_handles: Vec::new(),
        }
    }

    /// Start the service
    /// This will start all threads for the service to function properly
    pub fn start(&mut self) {
        if self.painter_input.is_some() {
            panic!("Can not start Service twice!")
        }
        let (painter_input, painter_output) = sync_channel(self.channel_limit);
        self.painter_input = Some(painter_input.clone());
        if self.converter_threads > 0 {
            let (merger_input, merger_output) = sync_channel(self.channel_limit);
            let mut distributor_output = Vec::new();
            for _ in 0..self.converter_threads {
                let (converter_input, converter_output) = sync_channel(self.channel_limit);
                self.join_handles.push(spawn(converter::get_converter(
                    self.image_config,
                    converter_output,
                    merger_input.clone(),
                )));
                distributor_output.push(converter_input);
            }
            let (converter_input, converter_output) = sync_channel(self.channel_limit);
            self.converter_input = Some(converter_input);
            self.join_handles
                .push(spawn(distributor::get_converter_distributor(
                    converter_output,
                    distributor_output,
                )));
            self.join_handles.push(spawn(merger::get_merger(
                merger_output,
                painter_input.clone(),
            )));
        }
        let mut painter_inputs = Vec::new();
        for i in 0..self.threads {
            let (painter_input, painter_output) = sync_channel(self.channel_limit);
            painter_inputs.push(painter_input);
            self.join_handles.push(spawn(painter::get_painter(
                painter_output,
                self.host.clone(),
                i,
                self.threads,
            )));
        }
        self.join_handles
            .push(spawn(distributor::get_painter_distributor(
                painter_output,
                painter_inputs,
            )));
    }

    fn start_check(&self) {
        if self.painter_input.is_none() {
            panic!("Service not started!")
        }
    }

    /// Change the image processing configuration
    /// WARNING: This will wait for all converter threads until their queue is
    /// empty enough
    pub fn change_image_config(&mut self, image_config: ImageConfig) {
        self.image_config = image_config;
        if let Some(converter_input) = &self.converter_input {
            let _ = converter_input.send(distributor::DistributorChange::Config(image_config));
        }
    }

    /// Send an image to be processed and painted afterwards
    pub fn send_image(&self, image: image::DynamicImage) {
        self.start_check();
        if let Some(converter_input) = &self.converter_input {
            let _ = converter_input.try_send(distributor::DistributorChange::Image(image));
        } else {
            panic!("Cannot send image without converter threads!")
        }
    }

    /// Send an image as commands to be painted
    pub fn send_command(&self, command: Arc<Command>) {
        self.start_check();
        let _ = self.painter_input.as_ref().unwrap().send(command);
    }

    /// Join all threads
    /// This will likely never exit if the service is running
    pub fn join(&mut self) {
        for handle in self.join_handles.drain(..) {
            let _ = handle.join();
        }
    }

    /// Stop the service and all associated threads
    pub fn stop(&mut self) {
        self.converter_input = None;
        self.painter_input = None;
        self.join();
    }

    /// Get a pixelfult client to send commands to the pixelflut server
    /// The client is automatically reconnected if the connection is lost
    /// WARNING: If servers have connection limits, this client will count as
    /// an additional client towards the limit
    pub fn get_client(&mut self) -> std::io::Result<&mut Client> {
        if let Some(client) = &self.worker_client {
            // If client is broken, reconnect
            if client.is_error() {
                let _ = client.shutdown();
                self.worker_client = None;
            }
        }
        if self.worker_client.is_none() {
            self.worker_client = Some(Client::new(self.host.new_stream()?));
        }
        Ok(self.worker_client.as_mut().unwrap())
    }

    /// Loop a callback for updating the image
    /// The callback is called until the service is stopped (via Service::stop)
    pub fn loop_callback<F>(&mut self, mut callback: F)
    where
        F: FnMut(&mut Service),
    {
        if self.painter_input.is_none() {
            self.start();
        }
        loop {
            callback(self);
            if self.painter_input.is_none() {
                break;
            }
        }
    }
}
