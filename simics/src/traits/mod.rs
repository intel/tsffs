use anyhow::Result;

use crate::project::Project;

pub trait Setup {
    fn setup(&self, project: &Project) -> Result<&Self>
    where
        Self: Sized;
}
