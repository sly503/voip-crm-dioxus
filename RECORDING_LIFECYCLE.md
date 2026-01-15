# Recording Lifecycle Integration

This document describes the call recording lifecycle integration in SipCall.

## Overview

The recording lifecycle is automatically managed through call state transitions:
- Recording **starts** when the call transitions to `Active` state (call is answered)
- Recording **stops** when the call transitions to `Ended` or `Failed` state
- Recorded audio is processed into a WAV file via `finalize_recording()`

## Usage

### 1. Enable Recording on a Call

Before setting up the RTP session, enable recording on the call:

```rust
let mut call = SipCall::new_outbound(
    call_id,
    sip_call_id,
    local_party,
    remote_party,
    event_tx,
);

// Enable recording for this call
call.enable_recording();
```

### 2. Set Up RTP Session with Recording

Create an RTP session with recording enabled:

```rust
let mut rtp_session = RtpSession::new(port, codec).await?;

// Enable recording on the RTP session
let recorder = rtp_session.enable_recording();

// Attach the RTP session to the call
// The recorder will be automatically extracted if recording is enabled
call.set_rtp_session(Arc::new(rtp_session));
```

### 3. Automatic Recording Start/Stop

Recording automatically starts and stops based on call state:

```rust
// When call is answered, recording starts automatically
call.set_state(CallState::Active).await;
// -> Recording is now capturing RTP packets

// When call ends, recording stops automatically
call.set_state(CallState::Ended).await;
// -> Recording has stopped
```

### 4. Finalize and Retrieve Recording

After the call ends, finalize the recording to get the WAV data:

```rust
match call.finalize_recording().await? {
    Some(wav_data) => {
        // Save to storage, upload to database, etc.
        save_recording(call_id, wav_data).await?;
    }
    None => {
        // No recording available (recording not enabled or no audio captured)
    }
}
```

## Recording Format

- **Audio Format**: WAV (PCM)
- **Sample Rate**: 8000 Hz
- **Channels**: 2 (Stereo)
  - Left channel: Outgoing audio (agent)
  - Right channel: Incoming audio (customer)
- **Bit Depth**: 16-bit signed PCM

## Architecture

### Components

1. **RtpRecorder** (`src/server/sip/rtp.rs`)
   - Captures RTP packets with direction and timestamp metadata
   - Provides start/stop control and packet buffering

2. **AudioMixer** (`src/server/sip/audio_mixer.rs`)
   - Decodes G.711 PCMU/PCMA audio from RTP packets
   - Aligns packets by timestamp
   - Mixes bidirectional audio into stereo output

3. **AudioConverter** (`src/server/sip/audio_converter.rs`)
   - Converts raw PCM samples to WAV file format
   - Validates parameters and handles errors

4. **SipCall** (`src/server/sip/call.rs`)
   - Manages recording lifecycle
   - Integrates all components
   - Provides high-level API

### Call Flow

```
1. enable_recording() called
   └─> recording_enabled = true

2. RTP session created with enable_recording()
   └─> RtpRecorder created

3. set_rtp_session() called
   └─> Recorder extracted and stored

4. Call state → Active
   └─> recorder.start() called
   └─> RTP packets captured with direction/timestamp

5. Audio transmitted bidirectionally
   └─> All packets captured in recorder buffer

6. Call state → Ended/Failed
   └─> recorder.stop() called

7. finalize_recording() called
   └─> Drain RTP packets from recorder
   └─> AudioMixer: Decode & align packets
   └─> AudioMixer: Mix to stereo PCM
   └─> AudioConverter: Convert PCM to WAV
   └─> Return WAV data
```

## Error Handling

- If recording is not enabled, `finalize_recording()` returns `Ok(None)`
- If no packets are captured, `finalize_recording()` returns `Ok(None)`
- If audio processing fails, `finalize_recording()` returns `Err(SipError::Codec(...))`
- Errors during start/stop are logged but don't fail the call

## Testing

Comprehensive tests are provided in `src/server/sip/call.rs`:

- `test_recording_enabled_flag` - Verify recording can be enabled
- `test_recording_lifecycle_without_recorder` - Handle missing recorder gracefully
- `test_recording_lifecycle_with_recorder` - Test start/stop functionality
- `test_recording_starts_on_active_state` - Auto-start on Active state
- `test_recording_stops_on_ended_state` - Auto-stop on Ended state
- `test_finalize_recording_without_enabled` - Handle disabled recording
- `test_finalize_recording_with_empty_packets` - Handle empty capture

Run tests:
```bash
cargo test --package voip-crm --lib sip::call::tests
```

## Next Steps

The recording lifecycle is now integrated into SipCall. The next subtasks will:

1. **Subtask 3.5**: Add consent announcement playback before recording
2. **Phase 4**: Integrate with database and storage system for persistence
3. **Phase 5**: Add UI for playback and management
4. **Phase 6**: Implement retention policies and compliance features

## Implementation Notes

- Recording uses stereo mode by default to keep agent and customer audio separate
- This allows for better analysis and quality assurance
- The recording buffer has a safety limit (100k packets) to prevent memory overflow
- All recording operations are logged for debugging and audit purposes
