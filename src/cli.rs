use clap::Command;

pub const NAME: &str = "metadata-preprocessor";

pub fn make_app() -> Command {
    Command::new(NAME).about("An mdbook preprocessor that parses markdown metadata")
}
