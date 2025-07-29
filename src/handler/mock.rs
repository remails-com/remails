//! A minimal mock-up for hickory_resolver

#[derive(Clone, Copy, Debug)]
pub struct Resolver {
    pub host: (&'static str, u16),
    pub txt: &'static str,
}

impl Resolver {
    pub async fn mx_lookup(
        &self,
        _: impl AsRef<str>,
    ) -> Result<[MX; 1], hickory_resolver::ResolveError> {
        Ok([MX(*self)])
    }

    pub async fn lookup_ip(
        &self,
        _: impl AsRef<str>,
    ) -> Result<[(); 1], hickory_resolver::ResolveError> {
        Ok([()])
    }

    pub async fn txt_lookup(
        &self,
        _: impl AsRef<str>,
    ) -> Result<[Txt; 1], hickory_resolver::ResolveError> {
        Ok([Txt(self.txt)])
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
        self.0.host.1
    }
}

pub struct ToStr(Resolver);

impl ToStr {
    pub fn to_utf8(&self) -> String {
        self.0.host.0.into()
    }
}
