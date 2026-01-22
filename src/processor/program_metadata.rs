use aion_program::prelude::{ProgramId, ProgramKeyId};

#[derive(Default)]
pub struct ProgramMetadata {
    program_id: Option<ProgramId>,
    program_key_id: Option<ProgramKeyId>,
}

impl ProgramMetadata {
    pub fn program_id(&self) -> &Option<ProgramId> {
        &self.program_id
    }

    pub fn program_key_id(&self) -> &Option<ProgramKeyId> {
        &self.program_key_id
    }
}