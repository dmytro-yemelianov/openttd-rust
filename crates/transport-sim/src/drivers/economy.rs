use crate::kernel::context::KernelContext;
use crate::kernel::driver::{DriverError, SubsystemDriver};
use crate::kernel::intent::KernelIntent;
use crate::kernel::phase::Phase;
use transport_types::{CapabilityToken, Money, ServiceId, ServicePriority};

/// Subsystem driver responsible for company finances, running costs, and maintenance.
pub struct EconomyDriver;

impl EconomyDriver {
    pub fn new() -> Self {
        Self
    }
}

impl Default for EconomyDriver {
    fn default() -> Self {
        Self::new()
    }
}

impl SubsystemDriver for EconomyDriver {
    fn id(&self) -> ServiceId {
        ServiceId::Economy
    }

    fn priority(&self) -> ServicePriority {
        ServicePriority::LOW
    }

    fn execute_phase(&mut self, phase: Phase, ctx: &mut KernelContext) -> Result<(), DriverError> {
        if phase != Phase::Drivers {
            return Ok(());
        }

        // Periodic running cost: every 100 ticks, charge 1 money per active vehicle
        if ctx.tick.0 > 0 && ctx.tick.0 % 100 == 0 {
            let companies: Vec<transport_types::CompanyID> =
                ctx.world.vehicles.values().map(|v| v.company_id).collect();
            for company_id in companies {
                ctx.stage_intent(
                    CapabilityToken::Company(company_id),
                    KernelIntent::DeductCost {
                        company_id,
                        amount: Money(1),
                    },
                );
            }
        }

        Ok(())
    }
}
