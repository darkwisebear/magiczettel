use std::{
    io::BufReader,
    str::FromStr,
};

use zettelwirtschaft::*;
use wasm_bindgen::prelude::*;

#[derive(Clone, Debug)]
enum ShoppingListResult {
    Nothing,
    Sorted(SortedZettel),
    ShoppingList(ShoppingList)
}

impl Default for ShoppingListResult {
    fn default() -> Self {
        Self::Nothing
    }
}

#[wasm_bindgen]
#[derive(Default, Clone, Debug)]
pub struct ShoppingListState {
    config: Option<Config>,
    result: ShoppingListResult,
}

impl ShoppingListState {
    fn goods_list_to_array(items: impl AsRef<[ShoppingListItem]>) -> js_sys::Array {
        items.as_ref().iter()
            .map(|item| JsValue::from(item.to_string()))
            .collect()
    }

    fn make_shopping_list_map(name: impl AsRef<str>, items: impl AsRef<[ShoppingListItem]>) -> js_sys::Object {
        let entry_name = JsValue::from_str("name");
        let entry_items = JsValue::from_str("items");

        let goods = Self::goods_list_to_array(items);
        let entry = js_sys::Object::new();
        js_sys::Reflect::set(&entry, &entry_name, &JsValue::from_str(name.as_ref())).unwrap();
        js_sys::Reflect::set(&entry, &entry_items, &JsValue::from(goods)).unwrap();
        entry
    }

    fn merchant_list_to_map(merchant_list: &MerchantList) -> js_sys::Object {
        Self::make_shopping_list_map(merchant_list.get_name(), merchant_list)
    }
}

#[wasm_bindgen]
impl ShoppingListState {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        console_error_panic_hook::set_once();
        Self::default()
    }

    pub fn process_input(&mut self, input: &str) {
        let names_mapping = self.config.as_ref().map(Config::make_alt_names_mapping)
            .transpose()
            .expect("Unable to create alt names mapping")
            .unwrap_or_default();
        let zettel = Zettel::from_buf_read(BufReader::new(input.as_bytes()))
            .expect("Unable to parse input file!");
        let sorted = SortedZettel::from_zettel(zettel, &names_mapping)
            .expect("Unable to sort zettel");

        if let Some(config) = &self.config {
            let shopping_list = ShoppingList::new(sorted, config)
                .expect("Unable to generate final shopping list");
            self.result = ShoppingListResult::ShoppingList(shopping_list);
        } else {
            self.result = ShoppingListResult::Sorted(sorted);
        }
    }

    pub fn get_plaintext_result(&self) -> String {
        match &self.result {
            ShoppingListResult::Nothing => "Nothing".to_string(),
            ShoppingListResult::Sorted(sorted) => sorted.to_string(),
            ShoppingListResult::ShoppingList(shopping_list) => shopping_list.to_string(),
        }
    }

    pub fn get_list_result(&self) -> js_sys::Array {
        match &self.result {
            ShoppingListResult::Nothing => js_sys::Array::new(),

            ShoppingListResult::Sorted(sorted) => {
                let entry = Self::make_shopping_list_map("Einkaufszettel", sorted);
                [entry].iter().map(JsValue::from).collect()
            }

            ShoppingListResult::ShoppingList(shopping_list) => {
                shopping_list.get_list().iter()
                    .map(Self::merchant_list_to_map)
                    .map(JsValue::from)
                    .collect()
            }
        }
    }

    pub fn load_config(&mut self, config: &str) {
        self.config = Some(Config::from_str(config).expect("Unable to load configuration"));
    }
}
