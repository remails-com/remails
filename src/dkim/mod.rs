use mail_auth::{common::headers::HeaderWriter, dkim::DkimSigner};

use crate::models::{Domain, MailAuthSigningKey};

pub struct PrivateKey<'a> {
    domain: &'a str,
    selector: &'a str,
    sign_key: MailAuthSigningKey,
    pub_key: aws_lc_rs::encoding::PublicKeyX509Der<'a>,
}

const SIGNED_HEADERS: [&str; 26] = [
    "From",
    "Subject",
    "Date",
    "Message-ID",
    "To",
    "Cc",
    "MIME-Version",
    "Content-Type",
    "Content-Transfer-Encoding",
    "Content-ID",
    "Content-Description",
    "Resent-Date",
    "Resent-From",
    "Resent-Sender",
    "Resent-To",
    "Resent-Cc",
    "Resent-Message-ID",
    "In-Reply-To",
    "References",
    "List-Id",
    "List-Help",
    "List-Unsubscribe",
    "List-Subscribe",
    "List-Post",
    "List-Owner",
    "List-Archive",
];

impl<'a> PrivateKey<'a> {
    pub fn new(domain: &'a Domain, selector: &'a str) -> Result<Self, crate::models::Error> {
        Ok(Self {
            domain: &domain.domain,
            selector,
            sign_key: domain.dkim_key.signing_key()?,
            pub_key: domain.dkim_key.pub_key()?,
        })
    }

    pub fn public_key(&self) -> &[u8] {
        self.pub_key.as_ref() // this is the full key including header info
    }

    pub fn dkim_header(self, msg: &mail_parser::Message) -> Result<String, mail_auth::Error> {
        let signer = DkimSigner::from_key(self.sign_key)
            .domain(self.domain)
            .selector(self.selector)
            .headers(SIGNED_HEADERS);

        signer.sign(&msg.raw_message).map(|x| x.to_header())
    }
}
