//! The `phonebook` area: phonebook entries and their groups. Both list reads
//! probe cleanly; the set/del writes are owned but run only at costly depth.

use async_trait::async_trait;

use crate::areas::probe_macros::probe_list;
use crate::harness::Report;
use crate::harness::area::{Area, AreaCtx, CostClass};
use voip_ms::*;

pub struct Phonebook;

#[async_trait(?Send)]
impl Area for Phonebook {
    fn name(&self) -> &'static str {
        "phonebook"
    }

    fn cost_class(&self) -> CostClass {
        CostClass::Free
    }

    fn methods(&self) -> &'static [&'static str] {
        &[
            "delPhonebook",
            "delPhonebookGroup",
            "getPhonebook",
            "getPhonebookGroups",
            "setPhonebook",
            "setPhonebookGroup",
        ]
    }

    async fn probe(&self, ctx: &AreaCtx<'_>, report: &mut Report) {
        const AREA: &str = "phonebook";

        probe_list!(
            ctx,
            report,
            AREA,
            "getPhonebook",
            GetPhonebookParams,
            GetPhonebookResponse,
            phonebooks
        );
        probe_list!(
            ctx,
            report,
            AREA,
            "getPhonebookGroups",
            GetPhonebookGroupsParams,
            GetPhonebookGroupsResponse,
            phonebooks
        );
    }
}
