use crate::app::command::CommandName;

#[derive(Debug, Clone)]
pub(crate) struct CommandInvocation {
    pub name: CommandName,
    #[allow(dead_code)]
    pub args: Vec<String>,
}
