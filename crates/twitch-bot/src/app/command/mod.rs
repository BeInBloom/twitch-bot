mod invocation;
mod name;
mod parser;

pub(crate) use invocation::CommandInvocation;
pub(crate) use name::CommandName;
pub(crate) use parser::CommandParser;

pub(crate) const MUSIC_COMMAND_NAME: &str = "music";
pub(crate) const SKIP_COMMAND_NAME: &str = "skip";
