use mail_auth::{
    common::{
        crypto::{RsaKey, Sha256},
        headers::HeaderWriter,
    },
    dkim::{DkimSigner, Done},
};

pub struct PrivateKey {
    signer: DkimSigner<RsaKey<Sha256>, Done>,
}

const _SIGNED_HEADER: [&str; 26] = [
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

impl PrivateKey {
    pub fn dkim_header(&self, msg: &[u8]) -> Result<String, mail_auth::Error> {
        self.signer.sign(msg).map(|x| x.to_header())
    }

    pub fn test_key(domain: &str, selector: &str) -> Result<Self, mail_auth::Error> {
        use std::io::Read;
        let mut pem = String::new();
        let _ = std::fs::File::open("dkim_key.pem")
            .unwrap()
            .read_to_string(&mut pem)
            .unwrap();

        let seckey = RsaKey::<Sha256>::from_pkcs8_pem(&pem)?;
        let signer = DkimSigner::from_key(seckey)
            .domain(domain)
            .selector(selector)
            .headers(["From", "Subject", "To"]);

        Ok(Self { signer })
    }
}

#[cfg(test)]
#[test]
fn dkim_header() {
    let rfc5322_message = "Hello world!".as_bytes();

    let signer = PrivateKey::test_key("example.com", "default").unwrap();

    // Sign an e-mail message using RSA-SHA256
    let signature_rsa = signer.dkim_header(rfc5322_message).unwrap();

    // Print the message including both signatures to stdout
    println!("{}", signature_rsa);
}
