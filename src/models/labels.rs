use derive_more::{Deref, Display};
use garde::Validate;
use regex::Regex;
use serde::{Deserialize, Deserializer, Serialize};
use std::str::FromStr;
use utoipa::{
    PartialSchema, ToSchema,
    openapi::{ObjectBuilder, RefOr, Schema, Type},
};

#[derive(Validate, Serialize, sqlx::Type, Display, Debug, Deref, PartialEq, Eq)]
#[serde(transparent)]
#[sqlx(transparent)]
#[garde(transparent)]
#[doc = include_str!("label_description.md")]
pub struct Label(#[garde(length(chars, min = 1, max = 30))] String);

impl PartialSchema for Label {
    fn schema() -> RefOr<Schema> {
        RefOr::from(Schema::Object(
            ObjectBuilder::new()
                .schema_type(Type::String)
                .max_length(Some(30))
                .min_length(Some(1))
                .description(Some(include_str!("label_description.md")))
                .build(),
        ))
    }
}

impl ToSchema for Label {}

impl Label {
    pub fn new(label: &str) -> Self {
        Self(Self::normalize(label))
    }

    fn normalize(label: &str) -> String {
        let regex = Regex::new(r"(\s|,)+").unwrap();
        regex
            .replace_all(&label.trim().to_lowercase(), "-")
            .to_string()
    }
}

impl FromStr for Label {
    type Err = garde::Report;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let l = Self::new(s);
        l.validate()?;
        Ok(l)
    }
}

impl<'de> Deserialize<'de> for Label {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let label = Label::new(&String::deserialize(deserializer)?);
        Ok(label)
    }
}

#[cfg(test)]
mod test {
    use crate::models::labels::Label;
    use garde::Validate;
    use utoipa::PartialSchema;

    #[test]
    fn deserialize_normalizes() {
        let json = serde_json::json!(" Default , \nLabel 3  #Ä\n");
        let label: Label = serde_json::from_value(json).unwrap();
        assert_eq!(&label.0, "default-label-3-#ä")
    }
    #[test]
    fn schema_shows_limits() {
        let schema = serde_json::to_value(Label::schema()).unwrap();
        let expected = serde_json::json!(
            {
              "minLength": 1,
              "maxLength": 30,
              "type": "string",
              "description": include_str!("label_description.md")
            }
        );
        assert_eq!(schema, expected);
    }

    #[test]
    fn validation() {
        // Note, because of the 'ä', the resulting label will be 31 bytes long, but only 30 characters
        let json_too_long_before_serialization =
            serde_json::json!(" Default , \nLabel 3  #Ä\n ad addddddd");
        let json_too_long_after_serialization =
            serde_json::json!(" Default , \nLabel 3  #Ä\n ad adddddddx");

        let label_too_long_before_serialization: Label =
            serde_json::from_value(json_too_long_before_serialization).unwrap();
        label_too_long_before_serialization
            .validate()
            .expect("validation should succeed");

        let label_too_long_after_serialization: Label =
            serde_json::from_value(json_too_long_after_serialization).unwrap();
        label_too_long_after_serialization
            .validate()
            .expect_err("validation should fail");
    }
}
