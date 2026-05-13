//! A minimal mock-up for hickory_resolver

use hickory_resolver::{
    lookup::Lookup,
    net::NetError,
    proto::{
        op::Query,
        rr::{Name, RData, Record, RecordType, rdata::{MX as MxRdata, TXT as TxtRdata}},
    },
};

#[derive(Clone, Debug)]
pub struct Resolver {
    pub host: (&'static str, u16),
    pub txt: Vec<&'static str>,
}

impl Resolver {
    pub async fn mx_lookup(&self, _: impl AsRef<str>) -> Result<Lookup, NetError> {
        let name = Name::from_ascii(self.host.0).unwrap_or(Name::root());
        let exchange = name.clone();
        let record = Record::from_rdata(name.clone(), 0, RData::MX(MxRdata::new(5, exchange)));
        let query = Query::query(name, RecordType::MX);
        Ok(Lookup::new_with_max_ttl(query, vec![record]))
    }

    pub async fn lookup_ip(&self, _: impl AsRef<str>) -> Result<[(); 1], NetError> {
        Ok([()])
    }

    pub async fn txt_lookup(&self, _: impl AsRef<str>) -> Result<Lookup, NetError> {
        let name = Name::root();
        let records: Vec<Record> = self
            .txt
            .iter()
            .filter(|txt| !txt.is_empty())
            .map(|txt| {
                let txt_data = TxtRdata::from_bytes(vec![txt.as_bytes()]);
                Record::from_rdata(name.clone(), 0, RData::TXT(txt_data))
            })
            .collect();
        let query = Query::query(name, RecordType::TXT);
        Ok(Lookup::new_with_max_ttl(query, records))
    }
}
