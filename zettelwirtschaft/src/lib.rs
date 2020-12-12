use std::{
    str::FromStr,
    ops::Add,
    io::BufRead,
    collections::HashMap,
};
use std::fmt::{self, Display, Formatter};

use failure::{Fallible, format_err, bail};
use itertools::Itertools;
use yaml_rust::{Yaml, YamlLoader};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Amount {
    Count(usize),
    Grams(usize),
    Millis(usize),
}

macro_rules! add_amount {
    ($self:expr, $other:expr, $($amount_variant:path),*) => {
        match $self {
            $($amount_variant(val) => if let $amount_variant(other_val) = $other {
                Ok($amount_variant(val + other_val))
            } else {
                Err(format_err!("Amount types differ"))
            },)*
        }
    }
}

impl Add for Amount {
    type Output = Fallible<Self>;

    fn add(self, other: Self) -> Self::Output {
        add_amount!(self, other, Amount::Count, Amount::Grams, Amount::Millis)
    }
}

macro_rules! convert_unit {
    ($val:expr, $unit:literal, $factor:literal, $amount_variant:path) => {
        if let Some(num_str) = $val.strip_suffix($unit) {
            f32::from_str(num_str)
                .ok()
                .map(|num| $amount_variant((num * $factor) as usize))
        } else {
            None
        }
    }
}

impl FromStr for Amount {
    type Err = failure::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        convert_unit!(s, "g", 1.0, Amount::Grams)
            .or_else(|| convert_unit!(s, "kg", 1000.0, Amount::Grams))
            .or_else(|| convert_unit!(s, "ml", 1.0, Amount::Millis))
            .or_else(|| convert_unit!(s, "l", 1000.0, Amount::Millis))
            .or_else(|| usize::from_str(s).ok().map(Amount::Count))
            .ok_or_else(|| format_err!("Unparsable amount {}", s))
    }
}

impl Display for Amount {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Grams(grams) => if *grams < 1000 {
                write!(f, "{}g", grams)
            } else {
                write!(f, "{}kg", *grams as f32 / 1000.0)
            }

            Self::Count(count) => write!(f, "{}", count),

            Self::Millis(milis) => if *milis < 1000 {
                write!(f, "{}ml", milis)
            } else {
                write!(f, "{}l", *milis as f32 / 1000.0)
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct ParsedLine {
    amount: Option<Amount>,
    name: String,
}

impl FromStr for ParsedLine {
    type Err = failure::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if !s.is_empty() {
            let count = s.split_whitespace().next().unwrap();
            let (amount, name) = if let Ok(amount) = Amount::from_str(count) {
                let name = s[count.len()..].trim();
                if name.is_empty() {
                    bail!("Line without name");
                }
                (Some(amount), name)
            } else {
                (None, s.trim())
            };

            let line = Self { amount, name: name.to_string() };
            Ok(line)
        } else {
            Err(format_err!("Empty line"))
        }
    }
}

#[derive(Debug, Clone)]
pub struct Zettel {
    zettel: Vec<ParsedLine>,
}

impl Zettel {
    pub fn from_buf_read<R: BufRead>(mut r: R) -> Fallible<Zettel> {
        let mut buf = String::new();
        let mut zettel = Vec::new();
        loop {
            match r.read_line(&mut buf) {
                Ok(0) => break Ok(Zettel { zettel }),
                Ok(_) => if buf.len() > 1 {
                    let line = ParsedLine::from_str(buf.as_str())?;
                    zettel.push(line);
                }
                Err(e) => break Err(e.into())
            }
            buf.clear();
        }
    }
}

#[derive(Debug, Clone)]
pub struct ShoppingListItem {
    amount: Amount,
    name: String,
}

impl Display for ShoppingListItem {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        if self.amount != Amount::Count(1) {
            write!(f, "{} {}", self.amount, self.name)
        } else {
            f.write_str(&self.name)
        }
    }
}

#[derive(Debug, Clone)]
pub struct SortedZettel {
    zettel: Vec<ShoppingListItem>
}

impl Display for SortedZettel {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        for item in &self.zettel {
            if item.amount == Amount::Count(1) {
                writeln!(f, "{}", &item.name)?;
            } else {
                writeln!(f, "{} {}", item.amount, &item.name)?;
            }
        }

        Ok(())
    }
}

impl SortedZettel {
    pub fn from_zettel(value: Zettel, alt_names_mapping: &AltNamesMapping<'_>) -> Fallible<Self> {
        let Zettel { mut zettel } = value;

        zettel.sort_by(|val1, val2| {
            let name1 = alt_names_mapping.normalize_name(val1.name.as_str());
            let name2 = alt_names_mapping.normalize_name(val2.name.as_str());
            name1.cmp(&name2)
        });
        let groups = zettel.into_iter()
            .group_by(|val| alt_names_mapping.normalize_name(val.name.as_str()).to_string());
        let zettel = (&groups).into_iter()
            .map(|(name, group)| {
                let mut amounts = group.map(|line| line.amount.unwrap_or(Amount::Count(1)));
                let start_amount = amounts.next().unwrap();
                let amount = amounts.try_fold(start_amount,
                                              |val, other| val + other);
                amount.map(|amount| ShoppingListItem { amount, name })
            }).collect::<Fallible<Vec<ShoppingListItem>>>()?;

        Ok(SortedZettel { zettel })
    }
}

impl AsRef<[ShoppingListItem]> for SortedZettel {
    fn as_ref(&self) -> &[ShoppingListItem] {
        self.zettel.as_slice()
    }
}

#[derive(Debug, Clone)]
pub struct MerchantList {
    name: String,
    items: Vec<ShoppingListItem>,
}

impl AsRef<[ShoppingListItem]> for MerchantList {
    fn as_ref(&self) -> &[ShoppingListItem] {
        self.items.as_slice()
    }
}

impl MerchantList {
    pub fn get_name(&self) -> &str {
        &self.name
    }
}

#[derive(Debug, Clone)]
pub struct ShoppingList {
    list: Vec<MerchantList>
}

const UNKNOWN_MERCHANT: &str = "Woanders";

impl ShoppingList {
    pub fn new(sorted: SortedZettel, config: &Config) -> Fallible<ShoppingList> {
        let mut sorted = sorted.zettel.into_iter()
            .map(|ShoppingListItem { name, amount }| (name, amount))
            .collect::<HashMap<_, _>>();

        let mut list = Vec::new();

        for merchant in config.merchant_iter()? {
            let MerchantConfig { name, goods } = merchant?;
            let mut items = Vec::new();
            for &good in &goods {
                if let Some((name, amount)) = sorted.remove_entry(good) {
                    items.push(ShoppingListItem { name, amount });
                }
            }
            if !items.is_empty() {
                list.push(MerchantList { name: name.to_string(), items })
            }
        }

        if !sorted.is_empty() {
            let items = sorted.into_iter()
                .map(|(name, amount)| ShoppingListItem { name, amount } )
                .collect::<Vec<_>>();
            list.push(MerchantList { name: UNKNOWN_MERCHANT.to_string(), items });
        }

        Ok(ShoppingList { list })
    }

    pub fn get_list(&self) -> &[MerchantList] {
        self.list.as_slice()
    }
}

impl Display for ShoppingList {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let mut first = true;
        for merchant_list in &self.list {
            if first {
                first = false;
            } else {
                f.write_str("\n\n")?;
            }

            writeln!(f, "{}", merchant_list.name)?;
            for _ in merchant_list.name.chars() {
                f.write_str("=")?;
            }
            f.write_str("\n")?;

            for item in &merchant_list.items {
                writeln!(f, "{}", item)?;
            }
        }

        Ok(())
    }
}

#[derive(Default, Clone, Debug)]
pub struct AltNamesMapping<'a>(HashMap<&'a str, &'a str>);

impl<'a> AltNamesMapping<'a> {
    fn new(waren_db: &'a Yaml) -> Fallible<Self> {
        let waren = waren_db["waren"].as_vec()
            .ok_or_else(|| format_err!("waren not existing or not an array"))?;

        let mut mapping = HashMap::new();

        for ware in waren {
            let canonical_name = ware["name"].as_str()
                .ok_or_else(|| format_err!("ware doesn't have a name"))?;

            match &ware["alt-names"] {
                Yaml::Array(alt_names) => {
                    for alt_name in alt_names {
                        let alt_name = alt_name.as_str()
                            .ok_or_else(|| format_err!("alt-name entry is not a string!"))?;
                        mapping.insert(alt_name, canonical_name);
                    }
                }

                Yaml::BadValue => (),

                _ => bail!("alt-names is not an array")
            }
        }

        Ok(Self(mapping))
    }

    fn normalize_name(&self, name: &'a str) -> &'a str {
        self.0.get(name).cloned().unwrap_or(name)
    }
}

#[derive(Debug, Clone)]
pub struct Config {
    config: Yaml
}

impl Config {
    pub fn new(config: Yaml) -> Self {
        Self { config }
    }

    pub fn make_alt_names_mapping(&self) -> Fallible<AltNamesMapping> {
        AltNamesMapping::new(&self.config)
    }

    pub fn merchant_iter(&self) -> Fallible<MerchantConfigIter> {
        self.config["locations"].as_vec()
            .ok_or_else(|| format_err!("locations doesn't exist or is not an array!"))
            .map(|locations| MerchantConfigIter(locations.iter()))
    }
}

pub struct MerchantConfig<'a> {
    name: &'a str,
    goods: Vec<&'a str>
}

pub struct MerchantConfigIter<'a>(std::slice::Iter<'a, Yaml>);

impl<'a> Iterator for MerchantConfigIter<'a> {
    type Item = Fallible<MerchantConfig<'a>>;

    fn next(&mut self) -> Option<Self::Item> {
        let raw_data = self.0.next()?;
        let name = match raw_data["name"].as_str() {
            Some(name) => name,
            None => return Some(Err(failure::err_msg("Name of merchant doesn't exist or \
                                                           is not a string"))),
        };
        let goods_array = match raw_data["waren"].as_vec() {
            Some(goods) => goods,
            None => return Some(Err(failure::err_msg("No goods associated with merchant")))
        };

        match goods_array.iter()
            .map(|item| item.as_str()
                .ok_or_else(|| failure::err_msg("Good in list is not a string")))
            .collect() {
            Ok(goods) => {
                let merchant_config = MerchantConfig { name, goods };
                Some(Ok(merchant_config))
            }

            Err(err) => Some(Err(err))
        }
    }
}

impl FromStr for Config {
    type Err = failure::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        YamlLoader::load_from_str(s)
            .map_err(failure::Error::from)
            .and_then(|yaml| yaml.into_iter().next()
                .ok_or_else(|| format_err!("No documents in YAML file")))
            .map(Config::new)
    }
}

#[cfg(test)]
mod tests {
    use std::io::BufReader;
    use super::*;

    #[test]
    fn add_grams() {
        let amount = "200g".parse::<Amount>().unwrap()
            .add("1.5kg".parse::<Amount>().unwrap())
            .unwrap();
        assert_eq!(Amount::Grams(1700), amount);
    }

    #[test]
    fn parse_zettel() {
        let sample = r#"500g Zucker
2l Milch

5 Eier
500ml Milch"#;
        let r = BufReader::new(sample.as_bytes());
        let zettel = Zettel::from_buf_read(r).unwrap();
        dbg!(&zettel);
        let sorted = SortedZettel::from_zettel(
            zettel, &AltNamesMapping::default()).unwrap();
        dbg!(sorted);
    }
}
