use std::fmt;

use eyre::{eyre, Context, Result};
use lazy_static::lazy_static;
use log::{debug, trace};
use scraper::{Html, Selector};

use crate::table::Table;

#[derive(Debug)]
pub struct Member {
    pub qq_name: String,
    pub group_name: String,
    pub qq_number: String,
    pub gender: Gender,
    pub qq_age: String,
    pub joined_date: String,
    pub last_spoken_date: String,
}

#[derive(Debug)]
pub enum Gender {
    Male,
    Female,
    Unknown,
}

impl fmt::Display for Gender {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Gender::Male => "男",
                Gender::Female => "女",
                Gender::Unknown => "未知",
            }
        )
    }
}

fn get_header<'a>(
    cell: &[&'a String],
    header: &'static str,
    row_index: usize,
    cell_index: usize,
) -> Result<&'a String> {
    cell.get(cell_index)
        .ok_or_else(|| {
            eyre!(format!(
                "Failed to get value for header `{header}`, at row `{row_index}`"
            ))
        })
        .map(|s| *s)
}

lazy_static! {
    static ref QQ_NAME_SLT: Selector = Selector::parse("span").unwrap();
    static ref GROUP_NAME_SLT: Selector = Selector::parse("span").unwrap();
}

impl Member {
    pub fn from_html(html: &str) -> Result<Vec<Self>> {
        trace!("---html---\n{:#?}", html);

        // let html_parsed = Html::parse_fragment(html);
        // let table_selector =
        //     Selector::parse("table").expect("failed to parse css selector (should not happen!)");

        // let html_tables: Vec<_> = html_parsed
        //     .select(&table_selector)
        //     .map(Table::new)
        //     .collect();

        // let mbr_slt = Selector::parse("tr.mb").expect("failed to parse tr selector");
        // info!(
        //     "Num members found: {}",
        //     html_parsed.select(&mbr_slt).count()
        // );

        // info!(
        //     "Table vec should have a len of 1. Actual len: {}",
        //     &html_tables.len()
        // );

        // let table = &html_tables
        //     .first()
        //     .wrap_err("Can't get first element of html table select")?;

        let table = Table::find_by_id(html, "groupMember")
            .ok_or_else(|| eyre!("Failed to extract table"))?;

        trace!("Table headers: {:?}", table.headers());

        // info!("Table {table:?}");

        let members: Vec<Member> = table
            .into_iter()
            .enumerate()
            .map(|(i, row)| {
                debug!("Row: {:#?}", &row);
                let cells: Vec<_> = row.iter().collect();

                /*
                 Example:
                         cells: [
                            "",
                            "1",
                            "<a class=\"group-master-a\"><i class=\"icon-group-master\"></i></a>\n\n                <img class=\"\" id=\"useIcon1452313818\" src=\"//q4.qlogo.cn/g?b=qq&amp;nk=1452313818&amp;s=140\">\n\n                <span> 秘书组 </span>",
                            "<span class=\"white\"> </span>",
                            "1452313818",
                            "男",
                            "11年",
                            "2018/02/26",
                            "2021/11/01",
                            "",
                        ]
                */

                Ok(Member {
                    qq_name: {
                        let name_raw_html = get_header(&cells, "成员", i, 2)?;
                        Html::parse_fragment(name_raw_html)
                            .select(&QQ_NAME_SLT)
                            .next()
                            .ok_or_else(|| {
                                eyre!(format!("Failed to find `成员` txt for elem {i}"))
                            })?
                            .inner_html()
                            .trim()
                            .to_owned()
                    },
                    group_name: {
                        let group_name_txt = get_header(&cells, "群昵称", i, 3)?;
                        let group_name = Html::parse_fragment(group_name_txt)
                            .select(&GROUP_NAME_SLT)
                            .next()
                            .ok_or_else(|| eyre!(format!("Failed to find `群昵称` for elem {i}")))?
                            .inner_html()
                            .trim()
                            .to_owned();

                        // if still has html, parse again
                        if group_name.starts_with('<') {
                            Html::parse_fragment(&group_name)
                                .select(&GROUP_NAME_SLT)
                                .next()
                                .ok_or_else(|| {
                                    eyre!(format!("Failed to find `群昵称` for elem {i}"))
                                })?
                                .inner_html()
                                .trim()
                                .to_owned()
                        } else {
                            group_name
                        }
                    },
                    qq_number: get_header(&cells, "QQ号", i, 4)?.to_owned(),
                    gender: match get_header(&cells, "性别", i, 5)?.as_str() {
                        "男" => Gender::Male,
                        "女" => Gender::Female,
                        "未知" => Gender::Unknown,
                        _ => panic!("Unrecognized Gender"),
                    },
                    qq_age: get_header(&cells, "Q龄", i, 6)?.to_owned(),
                    joined_date: get_header(&cells, "入群时间", i, 7)?.to_owned(),
                    last_spoken_date: get_header(&cells, "最后发言", i, 8)?.to_owned(),
                })
            })
            .collect::<Result<Vec<_>>>()
            .wrap_err("Failed to parse members")?;

        Ok(members)
    }
}
