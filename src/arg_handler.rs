use clap::{Arg, ArgAction, ArgMatches, Command};

pub fn parse() -> ArgMatches {
    // Handle/parse arguments
    Command::new("pixelbomber")
        .arg(
            Arg::with_name("HOST")
                .help("The host to pwn \"host:port\"")
                .required(true)
                .index(1),
        )
        .arg(
            Arg::with_name("image")
                .short('i')
                .long("image")
                .alias("images")
                .value_name("PATH")
                .help("Image paths")
                .required(true)
                .multiple(true)
                .display_order(1)
                .takes_value(true),
        )
        .arg(
            Arg::with_name("width")
                .short('w')
                .long("width")
                .value_name("PIXELS")
                .help("Draw width (def: screen width)")
                .display_order(2)
                .takes_value(true),
        )
        .arg(
            Arg::with_name("height")
                .short('h')
                .long("height")
                .value_name("PIXELS")
                .help("Draw height (def: screen height)")
                .display_order(3)
                .takes_value(true),
        )
        .arg(
            Arg::with_name("x")
                .short('x')
                .value_name("PIXELS")
                .help("Draw X offset (def: 0)")
                .display_order(4)
                .takes_value(true),
        )
        .arg(
            Arg::with_name("y")
                .short('y')
                .value_name("PIXELS")
                .help("Draw Y offset (def: 0)")
                .display_order(5)
                .takes_value(true),
        )
        .arg(
            Arg::with_name("count")
                .short('c')
                .long("count")
                .alias("thread")
                .alias("threads")
                .value_name("COUNT")
                .help("Number of concurrent threads (def: CPUs)")
                .display_order(6)
                .takes_value(true),
        )
        .arg(
            Arg::with_name("fps")
                .short('r')
                .long("fps")
                .value_name("RATE")
                .help("Frames per second with multiple images (def: 1)")
                .display_order(7)
                .takes_value(true),
        )
        .arg(
            Arg::with_name("feature_detection")
                .short('f')
                .long("--feature-detection")
                .help("disable automatic detection of supported features (def: true)")
                .display_order(8)
                .action(ArgAction::SetFalse),
        )
        .arg(
            Arg::with_name("offset")
                .short('o')
                .long("offset")
                .help("Whether or not the offset command is supported by the server (def: false)")
                .display_order(9)
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::with_name("gray")
                .short('g')
                .long("gray")
                .help("Whether or not the {PX x y gg} command is supported by the server (def: false)")
                .display_order(10)
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::with_name("alpha")
                .short('a')
                .long("alpha")
                .help("Whether or not the alpha value is used for pixels with alpha > 0 and < 255")
                .display_order(11)
                .action(ArgAction::SetTrue)
        )
        .get_matches()
}
