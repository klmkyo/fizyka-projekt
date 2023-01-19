use std::fs::{self, File};

use crate::{cellgrid::StationaryCharge, lib::helpers::XY};

pub struct MovableCharge {
    pub should_move: bool,
    pub collided: bool,
    pub x: f64,
    pub y: f64,
    pub q: f64,
    pub m: f64,
    pub v: XY<f64>,
    pub a: XY<f64>,
}

impl MovableCharge {
    pub fn vec_from_file(path: &str) -> Vec<MovableCharge> {
        // file format is
        // <x> <y> <q> <m> <vx> <vy> <ax> <ay>

        // read the file
        let file = fs::read_to_string(path)
            .unwrap_or_else(|_| panic!("Nie można odczytać pliku {}", path));
        // split the file into lines
        let lines = file.lines();
        // create a vector to store the movable charges
        let mut movable_charges = Vec::new();
        // iterate over the lines
        for (i, line) in lines.enumerate().filter(|(_, line)| !line.starts_with("#")) {
            let words: Vec<&str> = line.split_whitespace().collect();

            // check the number of words
            if words.len() != 8 {
                panic!("Nieprawidłowa liczba słów w linii {} pliku {}", i, path);
            }

            let movable_charge = MovableCharge {
                should_move: true,
                collided: false,
                x: words[0].parse().unwrap_or_else(|_| {
                    panic!("Nie można odczytać x w linii {} pliku {}", i, path)
                }),
                y: words[1].parse().unwrap_or_else(|_| {
                    panic!("Nie można odczytać y w linii {} pliku {}", i, path)
                }),
                q: words[2].parse().unwrap_or_else(|_| {
                    panic!("Nie można odczytać q w linii {} pliku {}", i, path)
                }),
                m: words[3].parse().unwrap_or_else(|_| {
                    panic!("Nie można odczytać m w linii {} pliku {}", i, path)
                }),
                v: XY {
                    x: words[4].parse().unwrap_or_else(|_| {
                        panic!("Nie można odczytać vx w linii {} pliku {}", i, path)
                    }),
                    y: words[5].parse().unwrap_or_else(|_| {
                        panic!("Nie można odczytać vy w linii {} pliku {}", i, path)
                    }),
                },
                a: XY {
                    x: words[6].parse().unwrap_or_else(|_| {
                        panic!("Nie można odczytać ax w linii {} pliku {}", i, path)
                    }),
                    y: words[7].parse().unwrap_or_else(|_| {
                        panic!("Nie można odczytać ay w linii {} pliku {}", i, path)
                    }),
                },
            };
            movable_charges.push(movable_charge);
        }

        movable_charges
    }
}

const K: f64 = 8.99e9;

// static mut lowest: f64 = INFINITY;

// This function calculates the field intensity at a point (x, y) caused by a
// set of stationary charges. The function returns an XY struct containing the
// field intnsity for x and y axis.
pub fn field_intensity_movable(
    x: f64,
    y: f64,
    stationary_charges: &Vec<StationaryCharge>,
) -> Option<XY<f64>> {
    let mut intensity_xy = XY { x: 0.0, y: 0.0 };
    for stationary_charge in stationary_charges {
        let r_sq =
            (x - stationary_charge.x as f64).powi(2) + (y - stationary_charge.y as f64).powi(2);
        let r = r_sq.sqrt();

        // If the distance between the given point and the stationary charge is
        // less than 2, the field intensity is goes way too high for accurate calculations.
        // We return infinity in this case, which later on is interpreted as a
        // collision of charges.
        if r < 2. {
            return None;
        }

        // code for debugging
        // if r < unsafe { lowest } {
        //     unsafe {
        //         lowest = r;
        //     }
        //     println!("lowest: {}", unsafe { lowest });
        // }

        // The electric field intensity (E) at a point caused by a stationary
        // charge is given by the formula E = k * q / r^3, where k is a constant,
        // q is the charge of the stationary charge, and r is the distance to the
        // point. We calculate the intensity of the field at the given point
        // caused by the given stationary charge and add it to the total intensity
        // vector.
        let factor = K * stationary_charge.q / (r_sq * r);
        intensity_xy.x += factor * (x - stationary_charge.x as f64);
        intensity_xy.y += factor * (y - stationary_charge.y as f64);

        // another way to calculate the intensity vector
        // get the angle of the intensity vector
        // let angle = (y - stationary_charge.y as f64).atan2(x - stationary_charge.x as f64);
        // calculate the intensity vector using trigonometry
        // let result2 = XY {
        //     x: intensity * angle.cos(),
        //     y: intensity * angle.sin(),
        // };
    }
    Some(intensity_xy)
}
