//! A minimal mock-up for hickory_resolver

#[derive(Clone, Copy, Debug)]
pub struct Resolver(pub &'static str, pub u16);

impl Resolver {
    pub async fn mx_lookup(
        &self,
        _: impl AsRef<str>,
    ) -> Result<[MX; 1], hickory_resolver::ResolveError> {
        Ok([MX(*self)])
    }

    pub async fn txt_lookup(
        &self,
        _: impl AsRef<str>,
    ) -> Result<[Txt; 1], hickory_resolver::ResolveError> {
        Ok([Txt(
            "v=DKIM1; k=rsa; p=MIIBIjANBgkqhkiG9w0BAQEFAAOCAQ8AMIIBCgKCAQEAr2nBRM/OsusiMxoYj8X1j7FOMD18IeGUGZ71RMZHCiCi1zrsSnwkOfaHA3WOeDZQZBABNqc2E1xoun8090A5mZuJGufXC4aMleqtBtY00af3n5l87d4wm4mI8bnZ66/sBNSqvGJNO1ccF6YrmPtVzBV52YjK4NjEa9J/B1vFfWbLMA8gslDF7qrqTupF0opf5So/iBQJjYZ8Pbv5wOmQtPs0t9jgaiq4ocqYYuLbR8B+hEM1PXgD4/kAeATAEsjGWNywC0kmpCb4L3hkBBn5sBusfJmFGNvBVUGb59oA6mkdFYBmTsAdDHmsREJS5tHKgh66GOMHDf6lhdJKAJeVwQIDAQAB",
        )])
    }
}

#[derive(Debug)]
pub struct Txt(pub &'static str);

impl Txt {
    pub fn txt_data(&self) -> [Vec<u8>; 1] {
        [self.0.as_bytes().to_vec()]
    }
}

#[derive(Debug)]
pub struct MX(Resolver);

impl MX {
    pub fn preference(&self) -> u16 {
        5
    }

    pub fn exchange(&self) -> ToStr {
        ToStr(self.0)
    }

    pub fn port(&self) -> u16 {
        self.0.1
    }
}

pub struct ToStr(Resolver);

impl ToStr {
    pub fn to_utf8(&self) -> String {
        self.0.0.into()
    }
}
