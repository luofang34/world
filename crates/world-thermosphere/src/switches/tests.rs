use super::*;

#[test]
fn the_standard_switches_turn_every_term_on() {
    let factors = Switches::STANDARD.factors(false);
    assert!(factors.sw.iter().skip(1).all(|&sw| sw == 1.0));
    assert!(factors.swc.iter().skip(1).all(|&swc| swc == 1.0));
}

#[test]
fn a_cross_terms_only_switch_keeps_the_cross_factor() {
    let switches = Switches::STANDARD.with(Effect::AsymmetricalAnnual, Switch::CrossTermsOnly);
    assert_eq!(
        switches.get(Effect::AsymmetricalAnnual),
        Switch::CrossTermsOnly
    );
    let factors = switches.factors(false);
    assert_eq!((factors.sw[5], factors.swc[5]), (0.0, 1.0));
    assert_eq!((factors.sw[6], factors.swc[6]), (1.0, 1.0));
}

#[test]
fn an_ap_history_flags_the_geomagnetic_term_only_when_it_is_on() {
    assert_eq!(Switches::STANDARD.factors(true).sw[9], -1.0);
    assert_eq!(Switches::STANDARD.factors(false).sw[9], 1.0);
    let off = Switches::STANDARD.with(Effect::GeomagneticActivity, Switch::Off);
    assert_eq!(off.factors(true).sw[9], 0.0);
}

#[test]
fn the_effects_are_listed_in_the_reference_order() {
    for (index, effect) in Effect::ALL.iter().enumerate() {
        assert_eq!(usize::from(effect.number()), index + 1);
    }
}
