/*
 * dtop: A tool for measuring system utilization of applications
 *       and system performance.
 *
 * SPDX-License-Identifier: GPL-2.0
 *
 * Author: Calinyara <mr.dengjie@gmail.com>
 */
 
use std::thread;
use thread_priority::*;
use std::sync::mpsc::{Sender, Receiver, TryRecvError};
use std::sync::mpsc;
use std::sync::{Mutex, Arc};
use tokio::prelude::*;
use tokio::timer::Interval;
use std::time::{Duration, Instant};
use std::fs::File;
use clap::{App, Arg, ArgMatches};
use itertools::izip;

const VERSION: &'static str = env!("CARGO_PKG_VERSION");
static PRIME: u64 = 7919;

#[derive(Clone, Debug, Copy)]
enum RunMode {
    AppUtilization,
    SysPerformance,
}

#[derive(Clone, Debug, Copy)]
struct Parameter {
    calibrating: bool,
    step_mode: bool,
    run_mode: RunMode,
    interval: i32,
}

fn main() {

    let matches = App::new("dtop")
        .version(VERSION)
        .about("dtop: A tool for measuring system utilization of applications and system performance")
        .author("Author: Calinyara <mr.dengjie@gmail.com>")
        .args_from_usage("-c, --calibrate 'Do Calibration'")
        .args_from_usage("-s，--step 'Step Mode'")
        .arg(Arg::from_usage("-m, --mode=[RUN MODE] '0: Application, 1: System Performance, By default 0'")
             .default_value("0")
            )
        .arg(Arg::from_usage("-i, --interval=[time as seconds] 'Specify the interval, by default 1 second'")
            .default_value("1")
        )
        .get_matches();

    let mut parameter = Parameter {
        calibrating: false,
        step_mode: false,
        run_mode: RunMode::AppUtilization,
        interval: 0
    };

    parse_parameters(&matches, &mut parameter);

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

    let mut counters: Vec<Arc<Mutex<i64>>> = Vec::with_capacity(core_num);
    for _ in 0..core_num {
        counters.push(Arc::new(Mutex::new(0)));
    }
    let mut counters_copy: Vec<Arc<Mutex<i64>>> = Vec::with_capacity(core_num);
    for i in 0..core_num {
        counters_copy.push(Arc::clone(&counters[i]));
    }

    let threads_info: Vec<_> = izip!(core_ids.into_iter(),
                                     channels.into_iter(),
                                     counters.into_iter()).collect();
    let handles = threads_info.into_iter().map(|info| {
        thread::spawn(move || {
            let (core_id, ch, counter) = (info.0, info.1, info.2);
            core_affinity::set_for_current(core_id);

            // println!("thread id {}", thread_native_id());
            match set_current_thread_priority(ThreadPriority::Min) {
                Err(why) => panic!("{:?}", why),
                Ok(_) => do_measure(&counter, ch),
            }
        })
    }).collect::<Vec<_>>();

    let mut run_times: u64 = 0;
    if parameter.calibrating || parameter.step_mode {
        run_times += 1;
    } else {
        run_times = u64::max_value();
    }

    let when = Instant::now() + Duration::from_secs(parameter.interval as u64);
    let task = Interval::new(when, Duration::from_secs(parameter.interval as u64))
        .take(run_times)
        .for_each(move |_| {
            let mut scores: Vec<i64> = vec![0; core_num];
            for i in 0..core_num {
                let mut num = counters_copy[i].lock().unwrap();
                scores[i] = *num;
                *num = 0;
            }

            for i in &mut scores {
                *i /= parameter.interval as i64;
            }

            if !parameter.calibrating {
                match File::open("scores.txt") {
                    Err(_) => {
                        let total_score = scores.iter().sum::<i64>();
                        println!("Calibrating...");
                        println!("Scores per CPU: {:?}", scores);
                        println!("Total Calibrated Score: {}\n", total_score);
                        save_calibration(total_score);
                    },
                    Ok(_) => {
                        let total_score = scores.iter().sum::<i64>();
                        let calibration_scores: f64 = get_calibration() as f64;
                        println!("Scores per CPU: {:?}", scores);
                        match parameter.run_mode {
                            RunMode::AppUtilization => {
                                let rate = (calibration_scores - total_score as f64) /  calibration_scores * 100.;
                                println!("Total Score: {}        System Utilization: {:7.3}%\n", total_score, rate);
                            },
                            RunMode::SysPerformance => {
                                let rate = total_score as f64 / calibration_scores * 100.;
                                println!("Total Score: {}        Performance Percentage: {:9.3}%\n", total_score, rate);
                            },
                        }
                    }
                }
            } else {
                let total_score = scores.iter().sum::<i64>();
                println!("Calibrating...");
                println!("Scores per CPU: {:?}", scores);
                println!("Total Calibrated Score: {}\n", total_score);
                save_calibration(total_score);
            }
            Ok(())
        })
        .map_err(|e| panic!("interval errored; err={:?}", e));

    tokio::run(task);

    if parameter.calibrating || parameter.step_mode {
        for tx_ch in tx_chs {
            tx_ch.send(parameter.interval).unwrap();
        }
    }

    for handle in handles.into_iter() {
        handle.join().unwrap();
    }
}

fn do_measure(c: &Arc<Mutex<i64>>, ch: (Sender<i32>, Receiver<i32>)) -> bool {
    loop {
        let r: bool = is_prime(PRIME);

        let mut num = c.lock().unwrap();
        *num += 1;

        match (ch.1).try_recv() {
            Ok(_) | Err(TryRecvError::Disconnected) => {
                break r;
            },
            Err(TryRecvError::Empty) => {},
        }
    }
}

fn parse_parameters(m: &ArgMatches, p: &mut Parameter) {
    if m.is_present("calibrate") {
        p.calibrating = true;
    }

    if  m.is_present("step") {
        p.step_mode = true;
    }

    p.run_mode = match m.value_of("mode").unwrap().parse::<i32>().unwrap() {
        1 => RunMode::SysPerformance,
        _ => RunMode::AppUtilization,
    };

    p.interval = m.value_of("interval").unwrap().parse::<i32>().unwrap();
}

fn save_calibration(t: i64) {
    let mut file = File::create("scores.txt").expect("create failed!");
    file.write_all(t.to_string().as_bytes()).expect("write failed!");
}

fn get_calibration() -> i64 {
    let mut file = File::open("scores.txt").unwrap();
    let mut s = String::new();
    file.read_to_string(&mut s).unwrap();
    s.parse::<i64>().unwrap()
}

fn is_prime(n: u64) -> bool {
    for a in 2..n {
        if n % a == 0 {
            return false;
        }
    }
    true
}
