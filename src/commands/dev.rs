use std::error::Error;

use super::DevArgs;
use crate::hyper::hyper;
use crate::test_robot::robot;

pub fn dev(args: DevArgs) -> Result<(), Box<dyn Error>> {
    match args.command {
        super::DevCommand::Robot(r) => robot(r),
        super::DevCommand::Hyper(h) => hyper(h),
    }
}
