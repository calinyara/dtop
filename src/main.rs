/*
 * SYSLOAD: a tool for measuring system load
 *
 * SPDX-License-Identifier: GPL-2.0
 *
 * Author: Calinyara <mr.dengjie@gmail.com>
 */
 
use core_affinity::*;
use std::thread;
use thread_priority::*;
use std::sync::mpsc::{Sender, Receiver, TryRecvError};
use std::sync::mpsc;
use tokio::prelude::*;
use tokio::timer::Delay;
use std::time::{Duration, Instant};
use std::fs::File;
use clap::App;

const VERSION: &'static str = env!("CARGO_PKG_VERSION");
static CALIBRATE_SECS: i32 = 1;

fn main() {

    let matches = App::new("sysload")
        .version(VERSION)
        .about("Evaluate The System Load!")
        .author("Author: Calinyara <mr.dengjie@gmail.com>")
        .args_from_usage("-c, --calibrate 'Do Calibration'")
        .get_matches();

    let core_ids = core_affinity::get_core_ids().unwrap();
    let core_num = core_ids.len();

    let mut channels: Vec<(Sender<i32>, Receiver<i32>)> = Vec::with_capacity(core_num);
    for _ in 0..core_num {
        channels.push(mpsc::channel());
    }

    let mut tx_chs: Vec<Sender<i32>> = Vec::with_capacity(core_num);
    for i in 0..core_num {
        tx_chs.push((channels[i].0).clone());
    }

    /*
     * Start a thread on each physical core and do scores statistics
     * info: (CoreId, (Sender<i32>, Receiver<i32>))
     */
    let threads_info: Vec<_> = core_ids.into_iter().zip(channels.into_iter()).collect();
    let handles = threads_info.into_iter().map(|info| {
        thread::spawn(move || {
            core_affinity::set_for_current(info.0);

            // println!("thread id {}", thread_native_id());
            match set_current_thread_priority(ThreadPriority::Min) {
                Err(why) => panic!("{:?}", why),
                Ok(_) => calibrate(info),
            }
        })
    }).collect::<Vec<_>>();

    /*
     * Notify the calibration threads after CALIBRATE_SECS secords
     */
    let when = Instant::now() + Duration::from_secs(CALIBRATE_SECS as u64);
    let task = Delay::new(when)
        .and_then(|_| {
            for tx_ch in tx_chs {
                tx_ch.send(CALIBRATE_SECS).unwrap();
            }

            Ok(())
        })
        .map_err(|e| panic!("delay errored; err={:?}", e));

    tokio::run(task);

    /*
     * System Scores Statistics
     */
    let mut scores = vec![];
    for handle in handles.into_iter() {
        let score = handle.join().unwrap();
        scores.push(score);
    }

    let total_score: i64 = scores.iter().sum::<u32>() as i64;
    if matches.is_present("calibrate") {
        println!("Calibrating...");
        println!("Scores per CPU: {:?}", scores);
        println!("Total Calibrated Score: {}", total_score);
        save_calibration(total_score);
    } else {
        match File::open("scores.txt") {
            Err(_) => {
                println!("Calibrating... Run again to check the system load!");
                println!("Scores per CPU: {:?}", scores);
                println!("Total Calibrated Score: {}", total_score);
                save_calibration(total_score);
            },
            Ok(_) => {
                let total_score: f64 = scores.iter().sum::<u32>() as f64;
                let calibration_scores: f64 = get_calibration() as f64;
                let sysload: f64 =  (calibration_scores - total_score) /  calibration_scores * 1000.;
                println!("System Load {:.3}%, Total Score: {}", sysload, total_score);
            }
        }
    }
}

fn calibrate(info: (CoreId, (Sender<i32>, Receiver<i32>))) -> u32 {

    let mut score: u32 = 0;

    loop {

        for _ in 1..1_000 {}

        score += 1;

        match ((info.1).1).try_recv() {
            Ok(_) | Err(TryRecvError::Disconnected) => {
                break score;
            },
            Err(TryRecvError::Empty) => {},
        }
    }
}

fn save_calibration(t: i64) {
    let mut file = File::create("scores.txt").expect("create failed");
    file.write_all(t.to_string().as_bytes()).expect("write failed");
}

fn get_calibration() -> i64 {
    let mut file = File::open("scores.txt").unwrap();
    let mut s = String::new();
    file.read_to_string(&mut s).unwrap();
    s.parse::<i64>().unwrap()
}
