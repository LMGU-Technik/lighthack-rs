use embassy_rp::{
    adc::{Adc, AdcPin, Async, Channel, Config, InterruptHandler},
    bind_interrupts,
    dma::AnyChannel,
    gpio::Pull,
    peripherals::ADC,
    usb::{Driver, Instance},
    Peripheral,
};
use embassy_time::{Duration, Ticker};
use embassy_usb::class::cdc_acm::CdcAcmClass;

use crate::{slip::encode_slip, Disconnected};

bind_interrupts!(struct Irqs {
    ADC_IRQ_FIFO => InterruptHandler;
});

pub fn setup(
    adc: impl Peripheral<P = ADC> + 'static,
    pan: impl Peripheral<P = impl AdcPin> + 'static,
    tilt: impl Peripheral<P = impl AdcPin> + 'static,
    dma: impl embassy_rp::dma::Channel + 'static,
) -> App<'static> {
    let adc = Adc::new(adc, Irqs, Config::default());
    let pan = Channel::new_pin(pan, Pull::None);
    let tilt = Channel::new_pin(tilt, Pull::None);
    App {
        adc,
        pan,
        tilt,
        dma: dma.degrade(),
    }
}

pub struct App<'d> {
    adc: Adc<'d, Async>,
    pan: Channel<'d>,
    tilt: Channel<'d>,
    dma: AnyChannel,
}

pub async fn pan_tilt<'d>(
    usb: &mut CdcAcmClass<'d, Driver<'d, impl Instance>>,
    app: &mut App<'static>,
) -> Result<(), Disconnected> {
    let mut ticker = Ticker::every(Duration::from_hz(10));
    loop {
        let pan = adc_oversample::<64>(app, PanTilt::Pan).await;
        let tilt = adc_oversample::<64>(app, PanTilt::Tilt).await;

        let packet: [u8; 6] = [0x01, 0x01, pan[0], pan[1], tilt[0], tilt[1]];

        let mut buf = [0u8; 11];

        let len = encode_slip(&packet, &mut buf);

        usb.write_packet(&buf[..len]).await?;

        ticker.next().await
    }
}

enum PanTilt {
    Pan,
    Tilt,
}

async fn adc_oversample<'d, const SAMPLES: usize>(app: &mut App<'d>, pt: PanTilt) -> [u8; 2] {
    let mut buf = [0u16; SAMPLES];
    app.adc
        .read_many(
            match pt {
                PanTilt::Pan => &mut app.pan,
                PanTilt::Tilt => &mut app.tilt,
            },
            &mut buf,
            1199,
            &mut app.dma,
        )
        .await
        .unwrap();

    let mut sum: usize = 0;

    for i in buf.iter() {
        sum += *i as usize;
    }

    let res = (sum / SAMPLES) as u16;

    res.to_le_bytes()
}
