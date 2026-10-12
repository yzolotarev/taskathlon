//! Catalogs: counterparties, items, employees and fixed assets.

use crate::money::Money;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Vat {
    /// Not subject to VAT.
    None,
    Zero,
    R10,
    R22,
}

impl Vat {
    pub fn percent(self) -> i64 {
        match self {
            Vat::None | Vat::Zero => 0,
            Vat::R10 => 10,
            Vat::R22 => 22,
        }
    }

    /// The VAT inside an amount that already includes it (amount × r / (100 + r)).
    pub fn inside(self, gross: Money) -> Money {
        let r = self.percent();
        gross.mul_div(r, 100 + r)
    }

    pub fn label(self) -> &'static str {
        match self {
            Vat::None => "Без НДС",
            Vat::Zero => "0%",
            Vat::R10 => "10%",
            Vat::R22 => "22%",
        }
    }

    pub const ALL: [Vat; 4] = [Vat::R22, Vat::R10, Vat::Zero, Vat::None];
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Counterparty {
    pub id: u32,
    pub name: String,
    pub inn: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Item {
    pub id: u32,
    pub name: String,
    pub unit: String,
    pub vat: Vat,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Employee {
    pub id: u32,
    pub name: String,
    pub position: String,
    pub salary: Money,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Asset {
    pub id: u32,
    pub name: String,
    /// Useful life for straight-line depreciation.
    pub life_months: u32,
}

/// The analytic a balance is broken down by (a "subconto").
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum Sub {
    Counterparty(u32),
    Item(u32),
    Employee(u32),
    Asset(u32),
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Catalogs {
    pub counterparties: Vec<Counterparty>,
    pub items: Vec<Item>,
    pub employees: Vec<Employee>,
    pub assets: Vec<Asset>,
}

impl Catalogs {
    pub fn counterparty(&self, id: u32) -> Option<&Counterparty> {
        self.counterparties.iter().find(|c| c.id == id)
    }

    pub fn item(&self, id: u32) -> Option<&Item> {
        self.items.iter().find(|c| c.id == id)
    }

    pub fn employee(&self, id: u32) -> Option<&Employee> {
        self.employees.iter().find(|c| c.id == id)
    }

    pub fn asset(&self, id: u32) -> Option<&Asset> {
        self.assets.iter().find(|c| c.id == id)
    }

    pub fn sub_name(&self, sub: Sub) -> String {
        match sub {
            Sub::Counterparty(id) => self.counterparty(id).map(|c| c.name.clone()),
            Sub::Item(id) => self.item(id).map(|c| c.name.clone()),
            Sub::Employee(id) => self.employee(id).map(|c| c.name.clone()),
            Sub::Asset(id) => self.asset(id).map(|c| c.name.clone()),
        }
        .unwrap_or_else(|| "<удалён>".into())
    }

    fn next(ids: impl Iterator<Item = u32>) -> u32 {
        ids.max().unwrap_or(0).saturating_add(1)
    }

    pub fn add_counterparty(&mut self, name: &str, inn: &str) -> u32 {
        let id = Self::next(self.counterparties.iter().map(|c| c.id));
        self.counterparties.push(Counterparty { id, name: name.into(), inn: inn.into() });
        id
    }

    pub fn add_item(&mut self, name: &str, unit: &str, vat: Vat) -> u32 {
        let id = Self::next(self.items.iter().map(|c| c.id));
        self.items.push(Item { id, name: name.into(), unit: unit.into(), vat });
        id
    }

    pub fn add_employee(&mut self, name: &str, position: &str, salary: Money) -> u32 {
        let id = Self::next(self.employees.iter().map(|c| c.id));
        self.employees.push(Employee { id, name: name.into(), position: position.into(), salary });
        id
    }

    pub fn add_asset(&mut self, name: &str, life_months: u32) -> u32 {
        let id = Self::next(self.assets.iter().map(|c| c.id));
        self.assets.push(Asset { id, name: name.into(), life_months: life_months.max(1) });
        id
    }
}
