use anyhow::{anyhow, Result};
use jack::{AudioIn, AudioOut, Client, ClientOptions, Control, Port, ProcessHandler, ProcessScope};
use std::sync::{Arc, Mutex};

pub struct AudioState {
    pub process_fn: Box<dyn FnMut(&[f32], &mut [f32], usize) + Send>,
}

pub struct JackEngine {
    sample_rate: usize,
    buffer_size: usize,
    client_name: String,
}

struct Handler {
    in_left: Port<AudioIn>,
    in_right: Port<AudioIn>,
    out_left: Port<AudioOut>,
    out_right: Port<AudioOut>,
    state: Arc<Mutex<AudioState>>,
}

impl ProcessHandler for Handler {
    fn process(&mut self, _client: &Client, ps: &ProcessScope) -> Control {
        let in_left: &[f32] = &self.in_left.as_slice(ps);
        let in_right: &[f32] = &self.in_right.as_slice(ps);
        let out_left: &mut [f32] = &mut self.out_left.as_mut_slice(ps);
        let out_right: &mut [f32] = &mut self.out_right.as_mut_slice(ps);

        let input: Vec<f32> = in_left
            .iter()
            .zip(in_right.iter())
            .flat_map(|(l, r)| [*l, *r])
            .collect();

        let nframes = ps.n_frames();
        let mut output = vec![0.0f32; input.len()];

        if let Ok(mut state) = self.state.lock() {
            (state.process_fn)(&input, &mut output, nframes as usize);
        }

        for (i, frame) in output.chunks_exact(2).enumerate() {
            out_left[i] = frame[0];
            out_right[i] = frame[1];
        }

        Control::Continue
    }
}

impl JackEngine {
    pub fn new(name: &str, state: Arc<Mutex<AudioState>>) -> Result<Self> {
        let (client, _status) = Client::new(name, ClientOptions::NO_START_SERVER)
            .map_err(|e| anyhow!("Failed to create JACK client: {:?}", e))?;

        let sample_rate = client.sample_rate() as usize;
        let buffer_size = client.buffer_size() as usize;
        let client_name = client.name().to_string();

        let in_left = client.register_port("in_left", AudioIn::default())?;
        let in_right = client.register_port("in_right", AudioIn::default())?;
        let out_left = client.register_port("out_left", AudioOut::default())?;
        let out_right = client.register_port("out_right", AudioOut::default())?;

        let handler = Handler {
            in_left,
            in_right,
            out_left,
            out_right,
            state,
        };

        let async_client = client
            .activate_async((), handler)
            .map_err(|e| anyhow!("Failed to activate JACK client: {:?}", e))?;

        let ports = async_client
            .as_client()
            .ports(None, Some("audio"), jack::PortFlags::IS_OUTPUT);

        let client = async_client.as_client();
        if let Some(input_port) = ports.first() {
            let port_name = format!("{}:in_left", client_name);
            client.connect_ports_by_name(input_port, &port_name)?;
        }
        if let Some(input_port) = ports.get(1) {
            let port_name = format!("{}:in_right", client_name);
            client.connect_ports_by_name(input_port, &port_name)?;
        } else if let Some(input_port) = ports.first() {
            let port_name = format!("{}:in_right", client_name);
            client.connect_ports_by_name(input_port, &port_name)?;
        }

        let playback_ports = client.ports(None, Some("audio"), jack::PortFlags::IS_INPUT);
        if let Some(output_port) = playback_ports.first() {
            let port_name = format!("{}:out_left", client_name);
            client.connect_ports_by_name(&port_name, output_port)?;
        }
        if let Some(output_port) = playback_ports.get(1) {
            let port_name = format!("{}:out_right", client_name);
            client.connect_ports_by_name(&port_name, output_port)?;
        } else if let Some(output_port) = playback_ports.first() {
            let port_name = format!("{}:out_right", client_name);
            client.connect_ports_by_name(&port_name, output_port)?;
        }

        std::mem::forget(async_client);

        Ok(Self {
            sample_rate,
            buffer_size,
            client_name,
        })
    }

    pub fn sample_rate(&self) -> usize {
        self.sample_rate
    }

    pub fn buffer_size(&self) -> usize {
        self.buffer_size
    }

    pub fn client_name(&self) -> &str {
        &self.client_name
    }
}
