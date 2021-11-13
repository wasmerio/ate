#[allow(unused_imports)]
use tracing::{info, warn, debug, error, trace};
use chrono::*;

use ate::prelude::*;
use crate::model::*;
use crate::error::*;

use super::*;

impl TokApi
{
    pub async fn get_or_create_this_month(&mut self) -> Result<DaoMut<HistoricMonth>, WalletError>
    {
        let now = chrono::offset::Utc::now().date();
        let mut wallet = self.wallet.as_mut();
        for month in wallet.history.iter_mut().await? {
            if now.month() == month.month && now.year() == month.year {
                return Ok(month);
            }
        }
        let ret = wallet.history.push(HistoricMonth {
            month: now.month(),
            year: now.year(),
            days: DaoVec::default(),
        })?;
        Ok(ret)
    }

    pub async fn get_or_create_today(&mut self) -> Result<DaoMut<HistoricDay>, WalletError>
    {
        let now = chrono::offset::Utc::now().date();
        let this_month = self.get_or_create_this_month().await?;
        for day in this_month.days.iter_mut_with_dio(&self.dio).await? {
            if day.day == now.day() {
                return Ok(day);
            }
        }
        let ret = this_month.days.push_with_dio(&self.dio, HistoricDay {
            day: now.day(),
            activities: Vec::new(),
        })?;
        Ok(ret)
    }

    pub async fn record_activity(&mut self, activity: HistoricActivity) -> Result<DaoMut<HistoricDay>, WalletError>
    {
        let mut today = self.get_or_create_today().await?;
        today.as_mut().activities.push(activity);
        Ok(today)
    }

    pub async fn read_activity(&mut self, filter_year: Option<i32>, filter_month: Option<u32>, filter_day: Option<u32>) -> Result<Vec<HistoricActivity>, WalletError>
    {
        let mut ret = Vec::new();
        for month in self.wallet.history.iter().await?
        {
            if let Some(a) = filter_year {
                if a != month.year {
                    continue;
                }
            }
            if let Some(a) = filter_month {
                if a != month.month {
                    continue;
                }
            }
            for day in month.days.iter().await? {
                if let Some(a) = filter_day {
                    if a != day.day {
                        continue;
                    }
                }
                
                let mut day = day.take();
                ret.append(&mut day.activities);
            }
        }

        ret.sort_by(|a, b| a.when().cmp(b.when()));

        Ok(ret)
    }
}