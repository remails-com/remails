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
        Ok([Txt(
            "v=DKIM1; k=rsa; p=MIIBIjANBgkqhkiG9w0BAQEFAAOCAQ8AMIIBCgKCAQEAyQtyx8uwJIJoQ3+LEetDzd+bpIkebVIYSq94OCOimHu/Pv7tPY5pn99JVv0rmdGHluuWEGxQNBYDBdk0FQF4+HP0MlPitJSdxawmCRsIcUZR3TQLf6dDBm2YPJ3G4xUQ2pT4GPMwCX9N1aAfO5qj2fBsjT8LvLeTRKEbHXGDM+m2yMF0dgr6AJLLVYjs3MSD273DEL5GnqhGXieziz4PI5TCJpxR3CVByguImG9tg1BySMu3f7VFmiToLCVeuk1UzIYAPZN6fvCcmyalADfG9rZa/60lxFzeorBtVk/Ej0braeX8AT8RX2Ozw9lg2Wzkwx5NyvqOFAcnkhDX4oTeVQIDAQAB",
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
