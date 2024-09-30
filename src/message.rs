use uuid::Uuid;


pub(crate) type EmailAddress = String;

pub(crate) struct Message {
    id: Uuid,
    from: EmailAddress,
    recipients: Vec<EmailAddress>,
    raw_message: Vec<u8>,
}

impl Message {
    pub fn new(from: EmailAddress) -> Self {
        let id = Uuid::new_v4();

        Self {
            id,
            from,
            recipients: Vec::new(),
            raw_message: Vec::new(),
        }
    }

    pub fn get_id(&self) -> Uuid {
        self.id
    }

    pub fn get_from(&self) -> &str {
        &self.from
    }

    pub fn add_recipient(&mut self, recipient: EmailAddress) {
        self.recipients.push(recipient);
    }

    pub fn set_raw_message(&mut self, raw_message: Vec<u8>) {
        self.raw_message = raw_message;
    }
}