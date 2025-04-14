use mail_auth::{
    common::{
        crypto::{RsaKey, Sha256},
        headers::HeaderWriter,
    },
    dkim::DkimSigner,
};

pub struct PrivateKey<'a> {
    domain: &'a str,
    selector: &'a str,
    sign_key: RsaKey<Sha256>,
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
    pub fn dkim_header(self, msg: &mail_parser::Message) -> Result<String, mail_auth::Error> {
        let signer = DkimSigner::from_key(self.sign_key)
            .domain(self.domain)
            .selector(self.selector)
            .headers(SIGNED_HEADERS);

        signer.sign(&msg.raw_message).map(|x| x.to_header())
    }

    pub fn test_key(domain: &'a str, selector: &'a str) -> Result<Self, mail_auth::Error> {
        use std::io::Read;
        let mut pem = String::new();
        let _ = std::fs::File::open("dkim_key.pem")
            .unwrap()
            .read_to_string(&mut pem)
            .unwrap();

        let sign_key = RsaKey::<Sha256>::from_pkcs8_pem(&pem)?;

        Ok(Self {
            sign_key,
            domain,
            selector,
        })
    }
}
