#[derive(Debug, Clone)]
pub(crate) struct CommandInvocation {
    pub name: String,
    pub args: Vec<String>,
}
