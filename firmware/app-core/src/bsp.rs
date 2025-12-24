use embassy_nrf::{
    Peripherals,
    config::{Config, DcdcConfig, Debug, HfclkSource, HfxoCapacitance, LfclkSource},
};

/// Initalize the chip
pub fn init() -> Peripherals {
    let mut config = Config::default();

    config.hfclk_source = HfclkSource::ExternalXtal;
    config.lfclk_source = LfclkSource::Synthesized;
    config.internal_capacitors = embassy_nrf::config::InternalCapacitors {
        hfxo: Some(HfxoCapacitance::_20_0pF),
        lfxo: None,
    };
    config.dcdc = DcdcConfig {
        regh: false,
        regmain: false,
        regradio: false,
        regh_voltage: None,
    };
    config.debug = Debug::Allowed;

    embassy_nrf::init(config)
}
