use rustc_hash::FxHashMap;
use windows::Win32::Graphics::Direct3D11::{D3D11_COMPARISON_NEVER, D3D11_FLOAT32_MAX, D3D11_SAMPLER_DESC, D3D11_TEXTURE_ADDRESS_MODE, ID3D11Device, ID3D11SamplerState};
use librashader_common::{FilterMode, WrapMode};
use crate::util::Result;
pub struct SamplerSet {
    samplers: FxHashMap<(WrapMode, FilterMode), ID3D11SamplerState>
}

impl SamplerSet {
    pub fn get(&self, wrap: WrapMode, filter: FilterMode) -> &ID3D11SamplerState {
        self.samplers.get(&(wrap, filter))
            .unwrap()
    }
    pub fn new(device: &ID3D11Device) -> Result<SamplerSet> {
        let mut samplers = FxHashMap::default();
        let wrap_modes =
            &[WrapMode::ClampToBorder, WrapMode::ClampToEdge, WrapMode::Repeat, WrapMode::MirroredRepeat];
        for wrap_mode in wrap_modes {
            unsafe {
                let linear = device.CreateSamplerState(&D3D11_SAMPLER_DESC {
                    Filter: FilterMode::Linear.into(),
                    AddressU: D3D11_TEXTURE_ADDRESS_MODE::from(*wrap_mode),
                    AddressV: D3D11_TEXTURE_ADDRESS_MODE::from(*wrap_mode),
                    AddressW: D3D11_TEXTURE_ADDRESS_MODE::from(*wrap_mode),
                    MipLODBias: 0.0,
                    MaxAnisotropy: 1,
                    ComparisonFunc: D3D11_COMPARISON_NEVER,
                    BorderColor: [0.0, 0.0, 0.0, 0.0],
                    MinLOD: -D3D11_FLOAT32_MAX,
                    MaxLOD: D3D11_FLOAT32_MAX,
                })?;

                let nearest = device.CreateSamplerState(&D3D11_SAMPLER_DESC {
                    Filter: FilterMode::Nearest.into(),
                    AddressU: D3D11_TEXTURE_ADDRESS_MODE::from(*wrap_mode),
                    AddressV: D3D11_TEXTURE_ADDRESS_MODE::from(*wrap_mode),
                    AddressW: D3D11_TEXTURE_ADDRESS_MODE::from(*wrap_mode),
                    MipLODBias: 0.0,
                    MaxAnisotropy: 1,
                    ComparisonFunc: D3D11_COMPARISON_NEVER,
                    BorderColor: [0.0, 0.0, 0.0, 0.0],
                    MinLOD: -D3D11_FLOAT32_MAX,
                    MaxLOD: D3D11_FLOAT32_MAX,
                })?;

                samplers.insert((*wrap_mode, FilterMode::Linear), linear);
                samplers.insert((*wrap_mode, FilterMode::Nearest), nearest);
            }
        }

        Ok(SamplerSet {
            samplers
        })
    }
}

