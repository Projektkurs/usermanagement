use bson::oid::ObjectId;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
#[derive(Default, Debug, Serialize, Deserialize)]
pub struct Layout {
    #[serde(rename = "_id")]
    id: ObjectId,
    name: String,
    layout: String,
    variables: Vec<String>,
}
impl Layout {
    pub fn create(name: String, layout: String, variables: Vec<String>) -> Option<Self> {
        if name == "" || layout == "" {
            return None;
        }
        Some(Layout {
            id: ObjectId::new(),
            name,
            layout,
            variables,
        })
    }
    pub fn resolve_variables(&self, variable_values: &HashMap<String, String>) -> Option<String> {
        let mut resolved_string = String::new();
        for variable in &self.variables {
            resolved_string = self
                .layout
                .replace(&format!("[{}]", &variable), &variable_values.get(variable)?);
        }
        Some(resolved_string)
    }
    /*async fn insert(&self, db: &mongodb::Client) -> Option<()> {
        if let Some(_room)= Room::getfromdb(&self.name, &db).await{
            return None
        }
        //todo
        if let Ok(_user) = db.layout_collection().insert_one(self, None).await {
            Some(())
        } else {
            None
        }
    }*/
}
