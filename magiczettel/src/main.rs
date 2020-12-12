use std::{
    fs::File,
    str::FromStr,
    path::PathBuf,
    io::{Read, Write, BufReader, BufWriter, stdin, stdout}
};

use zettelwirtschaft::*;

use structopt::StructOpt;

#[derive(StructOpt)]
struct Arguments {
    #[structopt(parse(from_os_str))]
    /// Unsorted list of goods to buy.
    ///
    /// Can be omitted to use stdin.
    input: Option<PathBuf>,
    #[structopt(parse(from_os_str))]
    /// Output file that emits the final shopping list.
    ///
    /// Can be omitted to use stdout.
    output: Option<PathBuf>,
    #[structopt(long, parse(from_os_str))]
    /// Path to a yaml file that lists goods.
    goods_db: Option<PathBuf>,
}

fn main() {
    let args = Arguments::from_args();

    let infile = if let Some(input) = args.input {
        let file = File::open(input)
            .expect("Unable to open input file");
        Box::new(file) as Box<dyn Read>
    } else {
        Box::new(stdin()) as Box<dyn Read>
    };

    let mut outfile = if let Some(output) = args.output {
        let file = File::create(output)
            .expect("Unable to open output file");
        Box::new(BufWriter::new(file)) as Box<dyn Write>
    } else {
        Box::new(stdout()) as Box<dyn Write>
    };

    let goods_db: Option<Config> = args.goods_db.map(|waren_db_path| {
        std::fs::read_to_string(waren_db_path)
            .map_err(failure::Error::from)
            .and_then(|s| Config::from_str(&s))
    }).transpose().expect("Unable to load wares DB");

    let zettel = Zettel::from_buf_read(BufReader::new(infile))
        .expect("Unable to parse input file");

    let alt_names_mapping = if let Some(goods_db) = goods_db.as_ref() {
        goods_db.make_alt_names_mapping()
            .expect("Unable to parse waren DB")
    } else {
        AltNamesMapping::default()
    };

    let sorted = SortedZettel::from_zettel(zettel, &alt_names_mapping)
        .expect("Failed to sort");
    if let Some(waren_db) = goods_db {
        let list = ShoppingList::new(sorted, &waren_db)
            .expect("Unable to map goods to merchants");
        write!(outfile, "{}", list).unwrap();
    } else {
        write!(outfile, "{}", sorted).unwrap();
    }
}
