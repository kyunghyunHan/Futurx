use egui_plot::{PlotPoint, PlotPoints};
use std::{
    sync::{
        mpsc::{self, Receiver, Sender},
        Arc, Mutex,
    },
    thread,
    time::{Duration, Instant},
};
use crate::utils::CandleData;
#[derive(Clone)]
pub enum StockType {
    DAY,
    YEAR1,
}

use std::collections::BTreeMap;
#[derive(Debug,Clone)]
pub struct MeasurementWindow {
    pub values: BTreeMap<u64, CandleData>,
    pub look_behind: usize,
    pub start_time: Instant,
    volumes: Vec<f64>, // Added volumes field
}
impl MeasurementWindow {
    pub fn new_with_look_behind(look_behind: usize, data: BTreeMap<u64, CandleData>) -> Self {
        Self {
            values: data,
            look_behind,
            start_time: Instant::now(),
            volumes: Vec::new(), // Initialize volumes
        }
    }
    pub fn add(&mut self, x: u64, candle: CandleData) {
        let now = Instant::now();
        let limit_time = now - Duration::from_secs(self.look_behind as u64);

        // Remove old values
        self.values.retain(|&key, _| {
            let timestamp = self.start_time + Duration::from_secs(key);
            timestamp >= limit_time
        });

        // Add new value
        self.values.insert(x, candle);
    }


    pub fn plot_values(&self) -> PlotPoints {
        PlotPoints::Owned(
            self.values
                .iter()
                .map(|(timestamp, candle)| PlotPoint::new(*timestamp as f64, candle.close))
                .collect(),
        )
    }

    pub fn volumes(&self) -> &Vec<f64> {
        &self.volumes
    }

    // pub fn high_price(&self) -> Option<f64> {
    //     self.values
    //         .values()
    //         .copied()
    //         .max_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
    // }
    pub fn highs(&self) -> Vec<(u64, f64)> {
        self.values
            .iter()
            .map(|(t, candle)| (*t, candle.high))
            .collect()
    }
    pub fn low_price(&self) -> Vec<(u64, f64)> {
        self.values.iter().map(|(t, candle)| (*t, candle.low)).collect()

    }

    // Helper method to get points as Vec for iteration
    pub fn get_points(&self) -> Vec<(u64, CandleData)> {
        self.values
            .iter()
            .map(|(&key, value)| (key, value.clone()))
            .collect()
    }

    // Add method to update volumes
    pub fn add_volume(&mut self, volume: f64) {
        self.volumes.push(volume);
    }
}
