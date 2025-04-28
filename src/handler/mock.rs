//! A minimal mock-up for hickory_resolver

#[derive(Clone, Copy, Debug)]
pub struct Resolver(pub &'static str, pub u16);

impl Resolver {
    pub async fn mx_lookup(&self, _: &str) -> Result<[MX; 1], hickory_resolver::ResolveError> {
        Ok([MX(*self)])
    }

    pub async fn txt_lookup(&self, _: &str) -> Result<[MX; 1], hickory_resolver::ResolveError> {
        todo!();
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
