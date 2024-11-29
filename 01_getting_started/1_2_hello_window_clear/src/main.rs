#![windows_subsystem = "windows"]

use common::{gfx::transition_barrier, os::App, util::print_debug_string};
use windows::{
    core::Interface,
    Win32::{
        Foundation::HANDLE,
        Graphics::{
            Direct3D::D3D_FEATURE_LEVEL_11_0,
            Direct3D12::{
                D3D12CreateDevice, D3D12GetDebugInterface, ID3D12CommandAllocator,
                ID3D12CommandQueue, ID3D12Debug, ID3D12DescriptorHeap, ID3D12Device, ID3D12Fence,
                ID3D12GraphicsCommandList, ID3D12InfoQueue, ID3D12Resource,
                D3D12_COMMAND_LIST_TYPE_DIRECT, D3D12_COMMAND_QUEUE_DESC,
                D3D12_CPU_DESCRIPTOR_HANDLE, D3D12_DESCRIPTOR_HEAP_DESC,
                D3D12_DESCRIPTOR_HEAP_TYPE_RTV, D3D12_FENCE_FLAG_NONE, D3D12_INFO_QUEUE_FILTER,
                D3D12_INFO_QUEUE_FILTER_DESC,
                D3D12_MESSAGE_ID_CLEARRENDERTARGETVIEW_MISMATCHINGCLEARVALUE,
                D3D12_MESSAGE_ID_MAP_INVALID_NULLRANGE, D3D12_MESSAGE_ID_UNMAP_INVALID_NULLRANGE,
                D3D12_MESSAGE_SEVERITY_CORRUPTION, D3D12_MESSAGE_SEVERITY_ERROR,
                D3D12_MESSAGE_SEVERITY_INFO, D3D12_MESSAGE_SEVERITY_WARNING,
                D3D12_RESOURCE_STATE_PRESENT, D3D12_RESOURCE_STATE_RENDER_TARGET,
            },
            Dxgi::{
                Common::{DXGI_FORMAT_R8G8B8A8_UNORM, DXGI_SAMPLE_DESC},
                CreateDXGIFactory2, DXGIGetDebugInterface1, IDXGIAdapter1, IDXGIDebug1,
                IDXGIFactory4, IDXGISwapChain3, DXGI_ADAPTER_FLAG, DXGI_ADAPTER_FLAG_NONE,
                DXGI_ADAPTER_FLAG_SOFTWARE, DXGI_CREATE_FACTORY_DEBUG, DXGI_CREATE_FACTORY_FLAGS,
                DXGI_DEBUG_ALL, DXGI_DEBUG_RLO_DETAIL, DXGI_DEBUG_RLO_IGNORE_INTERNAL,
                DXGI_MWA_NO_ALT_ENTER, DXGI_PRESENT, DXGI_SWAP_CHAIN_DESC1,
                DXGI_SWAP_EFFECT_FLIP_DISCARD, DXGI_USAGE_RENDER_TARGET_OUTPUT,
            },
        },
        System::Threading::{CreateEventA, WaitForSingleObject, INFINITE},
    },
};

const FRAME_COUNT: u32 = 2;

#[derive(Clone, Debug, Default)]
pub struct CommandLine {
    pub use_warp_device: bool,
}

pub fn build_command_line() -> CommandLine {
    let mut use_warp_device = false;

    for arg in std::env::args() {
        if arg.eq_ignore_ascii_case("-warp") || arg.eq_ignore_ascii_case("/warp") {
            use_warp_device = true;
        }
    }

    CommandLine { use_warp_device }
}

fn get_hardware_adapter(factory: &IDXGIFactory4) -> windows::core::Result<IDXGIAdapter1> {
    for i in 0.. {
        let adapter = unsafe { factory.EnumAdapters1(i) }?;
        let desc = unsafe { adapter.GetDesc1() }?;

        if (DXGI_ADAPTER_FLAG(desc.Flags as _) & DXGI_ADAPTER_FLAG_SOFTWARE)
            != DXGI_ADAPTER_FLAG_NONE
        {
            // Don't select the Basic Render Driver adapter.
            // Pass in "/warp" on the command line if you want a software adapter.
            continue;
        }

        // Check to see whether the adapter supports D3D12 but don't create the device yet.
        if unsafe {
            D3D12CreateDevice(
                &adapter,
                D3D_FEATURE_LEVEL_11_0,
                std::ptr::null_mut::<Option<ID3D12Device>>(),
            )
        }
        .is_ok()
        {
            return Ok(adapter);
        }
    }

    unreachable!()
}

fn create_device(
    command_line: &CommandLine,
) -> Result<(IDXGIFactory4, ID3D12Device), Box<dyn std::error::Error>> {
    if cfg!(debug_assertions) {
        unsafe {
            let mut debug: Option<ID3D12Debug> = None;
            if let Some(debug) = D3D12GetDebugInterface(&mut debug).ok().and(debug) {
                debug.EnableDebugLayer();

                if let Ok(dxgi_debug) = DXGIGetDebugInterface1::<IDXGIDebug1>(0) {
                    dxgi_debug.EnableLeakTrackingForThread();
                }
            }
        }
    }

    let dxgi_factory_flags = if cfg!(debug_assertions) {
        DXGI_CREATE_FACTORY_DEBUG
    } else {
        DXGI_CREATE_FACTORY_FLAGS(0)
    };

    let dxgi_factory: IDXGIFactory4 = unsafe { CreateDXGIFactory2(dxgi_factory_flags) }?;

    let adapter = if command_line.use_warp_device {
        unsafe { dxgi_factory.EnumWarpAdapter() }
    } else {
        get_hardware_adapter(&dxgi_factory)
    }?;

    let mut device: Option<ID3D12Device> = None;
    unsafe { D3D12CreateDevice(&adapter, D3D_FEATURE_LEVEL_11_0, &mut device) }?;
    let device: ID3D12Device = device.ok_or("failed to create device")?;

    if cfg!(debug_assertions) {
        unsafe {
            let info_queue = device.cast::<ID3D12InfoQueue>()?;
            info_queue.SetBreakOnSeverity(D3D12_MESSAGE_SEVERITY_CORRUPTION, true)?;
            info_queue.SetBreakOnSeverity(D3D12_MESSAGE_SEVERITY_ERROR, true)?;
            info_queue.SetBreakOnSeverity(D3D12_MESSAGE_SEVERITY_WARNING, true)?;

            let mut severities = [D3D12_MESSAGE_SEVERITY_INFO];
            let mut deny_ids = [
                D3D12_MESSAGE_ID_CLEARRENDERTARGETVIEW_MISMATCHINGCLEARVALUE,
                D3D12_MESSAGE_ID_MAP_INVALID_NULLRANGE,
                D3D12_MESSAGE_ID_UNMAP_INVALID_NULLRANGE,
            ];

            let filter = D3D12_INFO_QUEUE_FILTER {
                DenyList: D3D12_INFO_QUEUE_FILTER_DESC {
                    NumSeverities: severities.len() as u32,
                    pSeverityList: severities.as_mut_ptr(),
                    NumIDs: deny_ids.len() as u32,
                    pIDList: deny_ids.as_mut_ptr(),
                    ..Default::default()
                },
                ..Default::default()
            };

            info_queue.PushStorageFilter(&filter)?;
        }
    }

    Ok((dxgi_factory, device))
}

fn report_live_objects() {
    unsafe {
        if cfg!(debug_assertions) {
            if let Ok(dxgi_debug) = DXGIGetDebugInterface1::<IDXGIDebug1>(0) {
                let _ = dxgi_debug.ReportLiveObjects(
                    DXGI_DEBUG_ALL,
                    DXGI_DEBUG_RLO_DETAIL | DXGI_DEBUG_RLO_IGNORE_INTERNAL,
                );
            }
        }
    }
}

#[allow(unused)]
struct GpuResources {
    dxgi_factory: IDXGIFactory4,
    device: ID3D12Device,
    command_queue: ID3D12CommandQueue,
    swapchain: IDXGISwapChain3,
    frame_index: u32,
    rtv_heap: ID3D12DescriptorHeap,
    rtv_descriptor_size: usize,
    rtv_handle: D3D12_CPU_DESCRIPTOR_HANDLE,
    render_targets: [ID3D12Resource; FRAME_COUNT as usize],
    command_allocator: ID3D12CommandAllocator,
    command_list: ID3D12GraphicsCommandList,
    fence: ID3D12Fence,
    fence_value: u64,
    fence_event: HANDLE,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let command_line = build_command_line();

    let mut title = "Hello Window Clear".to_string();
    if command_line.use_warp_device {
        title.push_str(" (WARP)");
    }

    let (mut app, window) = App::init(title, (800, 600))?;

    // Adapter.
    let (dxgi_factory, device) = create_device(&command_line)?;

    // Resources.
    let command_queue: ID3D12CommandQueue = unsafe {
        device.CreateCommandQueue(&D3D12_COMMAND_QUEUE_DESC {
            Type: D3D12_COMMAND_LIST_TYPE_DIRECT,
            ..Default::default()
        })
    }?;

    let (width, height) = window.get_physical_size();

    let swapchain_desc = DXGI_SWAP_CHAIN_DESC1 {
        BufferCount: FRAME_COUNT,
        Width: width as u32,
        Height: height as u32,
        Format: DXGI_FORMAT_R8G8B8A8_UNORM,
        BufferUsage: DXGI_USAGE_RENDER_TARGET_OUTPUT,
        SwapEffect: DXGI_SWAP_EFFECT_FLIP_DISCARD,
        SampleDesc: DXGI_SAMPLE_DESC {
            Count: 1,
            ..Default::default()
        },
        ..Default::default()
    };

    // todo: resize swapchain when window is resized.
    let swapchain: IDXGISwapChain3 = unsafe {
        dxgi_factory.CreateSwapChainForHwnd(
            &command_queue,
            window.get_handle(),
            &swapchain_desc,
            None,
            None,
        )
    }?
    .cast()?;

    // todo: support fullscreen transitions and escape to exit window.
    unsafe {
        dxgi_factory.MakeWindowAssociation(window.get_handle(), DXGI_MWA_NO_ALT_ENTER)?;
    }

    let frame_index = unsafe { swapchain.GetCurrentBackBufferIndex() };

    let rtv_heap: ID3D12DescriptorHeap = unsafe {
        device.CreateDescriptorHeap(&D3D12_DESCRIPTOR_HEAP_DESC {
            NumDescriptors: FRAME_COUNT,
            Type: D3D12_DESCRIPTOR_HEAP_TYPE_RTV,
            ..Default::default()
        })
    }?;

    let rtv_descriptor_size =
        unsafe { device.GetDescriptorHandleIncrementSize(D3D12_DESCRIPTOR_HEAP_TYPE_RTV) } as usize;

    let rtv_handle = unsafe { rtv_heap.GetCPUDescriptorHandleForHeapStart() };

    let render_targets: [ID3D12Resource; FRAME_COUNT as usize] = core::array::from_fn(|i| {
        let render_target: ID3D12Resource = match unsafe { swapchain.GetBuffer(i as u32) } {
            Ok(render_target) => render_target,
            Err(e) => {
                if cfg!(debug_assertions) {
                    panic!("{e}")
                } else {
                    panic!("failed to obtain render target for swapchain buffer {i}")
                }
            }
        };
        unsafe {
            device.CreateRenderTargetView(
                &render_target,
                None,
                D3D12_CPU_DESCRIPTOR_HANDLE {
                    ptr: rtv_handle.ptr + i * rtv_descriptor_size,
                },
            )
        };
        render_target
    });

    let command_allocator: ID3D12CommandAllocator =
        unsafe { device.CreateCommandAllocator(D3D12_COMMAND_LIST_TYPE_DIRECT) }?;

    let command_list: ID3D12GraphicsCommandList = unsafe {
        // todo: initial state PSO get's passed here instead of None.
        device.CreateCommandList(0, D3D12_COMMAND_LIST_TYPE_DIRECT, &command_allocator, None)
    }?;
    unsafe { command_list.Close() }?;

    let fence: ID3D12Fence = unsafe { device.CreateFence(0, D3D12_FENCE_FLAG_NONE) }?;

    let fence_value: u64 = 1;

    let fence_event = unsafe { CreateEventA(None, false, false, None) }?;

    let mut resouces = GpuResources {
        dxgi_factory,
        device,
        command_queue,
        swapchain,
        frame_index,
        rtv_heap,
        rtv_descriptor_size,
        rtv_handle,
        render_targets,
        command_allocator,
        command_list,
        fence,
        fence_value,
        fence_event,
    };

    // Run main loop.

    while app.run() {
        render(&mut resouces);
    }

    std::mem::drop(resouces);

    report_live_objects();

    Ok(())
}

// Example related graphics.
fn populate_command_list(resources: &GpuResources) -> windows::core::Result<()> {
    // Command list allocators can only be reset when the associated
    // command lists have finished execution on the GPU; apps should use
    // fences to determine GPU execution progress.
    unsafe { resources.command_allocator.Reset() }?;

    // However, when ExecuteCommandList() is called on a particular
    // command list, that command list can then be reset at any time and
    // must be before re-recording.
    unsafe {
        resources
            .command_list
            .Reset(&resources.command_allocator, None)
    }?;

    // Indicate that the back buffer will be used as a render target.
    let barrier = transition_barrier(
        &resources.render_targets[resources.frame_index as usize],
        D3D12_RESOURCE_STATE_PRESENT,
        D3D12_RESOURCE_STATE_RENDER_TARGET,
    );
    unsafe {
        resources.command_list.ResourceBarrier(&[barrier]);
    }

    let rtv_handle = D3D12_CPU_DESCRIPTOR_HANDLE {
        ptr: unsafe { resources.rtv_heap.GetCPUDescriptorHandleForHeapStart() }.ptr
            + resources.frame_index as usize * resources.rtv_descriptor_size,
    };

    unsafe {
        resources
            .command_list
            .OMSetRenderTargets(1, Some(&rtv_handle), false, None);
    }

    // Record commands.
    unsafe {
        resources
            .command_list
            .ClearRenderTargetView(rtv_handle, &[0.0, 0.2, 0.4, 1.0], None);

        // Indicate that the back buffer will now be used to present.
        let barrier = transition_barrier(
            &resources.render_targets[resources.frame_index as usize],
            D3D12_RESOURCE_STATE_RENDER_TARGET,
            D3D12_RESOURCE_STATE_PRESENT,
        );
        resources.command_list.ResourceBarrier(&[barrier]);
    }

    unsafe { resources.command_list.Close() }
}

fn wait_for_previous_frame(resources: &mut GpuResources) {
    // todo: THIS IS NOT BEST PRACTICE BUT IT IS EXPEDIENT FOR NOW!
    let current_fence_value = resources.fence_value;

    if let Err(e) = unsafe {
        resources
            .command_queue
            .Signal(&resources.fence, current_fence_value)
    } {
        print_debug_string(&format!("failed to signal fence {e}"));
    }

    resources.fence_value += 1;

    // Wait until the previous frame is finished.
    if unsafe { resources.fence.GetCompletedValue() } < current_fence_value {
        if let Err(e) = unsafe {
            resources
                .fence
                .SetEventOnCompletion(current_fence_value, resources.fence_event)
        } {
            print_debug_string(&format!("failed to set fence event completion value {e}"));
        }

        unsafe { WaitForSingleObject(resources.fence_event, INFINITE) };
    }

    resources.frame_index = unsafe { resources.swapchain.GetCurrentBackBufferIndex() };
}

fn render(resources: &mut GpuResources) {
    if let Err(e) = populate_command_list(resources) {
        print_debug_string(&format!("failed to populate command list {e}"));
        return;
    }

    // Execute the command list.
    let command_list = Some(resources.command_list.cast().unwrap()); // todo: no unwrap.
    unsafe {
        resources.command_queue.ExecuteCommandLists(&[command_list]);
    };

    // Present the frame.
    if let Err(e) = unsafe { resources.swapchain.Present(1, DXGI_PRESENT(0)) }.ok() {
        print_debug_string(&format!("failed to present the frame {e}"));
        return;
    }

    wait_for_previous_frame(resources);
}
