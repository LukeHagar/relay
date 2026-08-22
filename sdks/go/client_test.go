// Integration tests for the Go SDK. Skipped unless RELAY_TEST_URL is set and
// reachable (start an engine, then: RELAY_TEST_URL=http://127.0.0.1:8000 go test ./...)
package relay

import (
	"context"
	"encoding/json"
	"os"
	"strings"
	"testing"
	"time"
)

func liveClient(t *testing.T) (*Client, string) {
	t.Helper()
	base := os.Getenv("RELAY_TEST_URL")
	if base == "" {
		t.Skip("RELAY_TEST_URL not set; skipping live-engine tests")
	}
	c := New(base, os.Getenv("RELAY_TEST_TOKEN"))
	ctx, cancel := context.WithTimeout(context.Background(), 3*time.Second)
	defer cancel()
	if err := c.do(ctx, "GET", "/healthz", nil, nil); err != nil {
		t.Skipf("engine at %s unreachable: %v", base, err)
	}
	return c, base
}

func TestChannelRoundTrip(t *testing.T) {
	c, _ := liveClient(t)
	ctx := context.Background()
	ch := "sdk-go"

	if err := c.UpsertChannel(ctx, ch, ChannelConfig{ResponseMode: "echo"}); err != nil {
		t.Fatalf("upsert: %v", err)
	}

	// capture through the capture plane
	capture := c.CaptureURL(ch) + "/evt1?a=1"
	req, _ := httpNewRequest("POST", capture, `{"hello":"go-sdk"}`)
	resp, err := c.HTTP.Do(req)
	if err != nil {
		t.Fatalf("capture: %v", err)
	}
	resp.Body.Close()
	if resp.StatusCode != 200 {
		t.Fatalf("capture status: %d", resp.StatusCode)
	}

	events, err := c.History(ctx, ch, 0, 100)
	if err != nil {
		t.Fatalf("history: %v", err)
	}
	if len(events) == 0 {
		t.Fatal("expected at least one event")
	}
	env := events[len(events)-1]
	if env.Query == nil || *env.Query != "a=1" {
		t.Fatalf("query fidelity lost: %v", env.Query)
	}
	var body map[string]any
	if err := json.Unmarshal([]byte(env.Body), &body); err != nil || body["hello"] != "go-sdk" {
		t.Fatalf("body fidelity lost: %s", env.Body)
	}
}

func TestReplayTypedError(t *testing.T) {
	c, _ := liveClient(t)
	_, err := c.Replay(context.Background(), ReplayRequest{
		TargetURL: "http://127.0.0.1:8001",
		Source:    ReplaySource{EventID: strPtr("01NOPE")},
	})
	if err == nil {
		t.Fatal("expected error for missing event")
	}
	var relayErr *Error
	if !asRelayError(err, &relayErr) || relayErr.Code != "no_such_event" {
		t.Fatalf("expected typed no_such_event, got: %v", err)
	}
}

func TestOnceReceivesLiveEvent(t *testing.T) {
	c, _ := liveClient(t)
	ctx, cancel := context.WithTimeout(context.Background(), 5*time.Second)
	defer cancel()

	if err := c.UpsertChannel(ctx, "sdk-go-once", ChannelConfig{ResponseMode: "echo"}); err != nil {
		t.Fatalf("upsert: %v", err)
	}

	// Once runs produce() only after the engine confirms the subscription, so the
	// produced capture can never race the handshake — deterministic, no sleeps.
	env, err := c.Once(ctx, "sdk-go-once", "newest", func() error {
		resp, err := c.HTTP.Post(c.CaptureURL("sdk-go-once")+"/live", "text/plain",
			strings.NewReader("ping"))
		if err != nil {
			return err
		}
		return resp.Body.Close()
	})
	if err != nil {
		t.Fatalf("once: %v", err)
	}
	if env.Path != "/live" || env.Body != "ping" {
		t.Fatalf("unexpected event: %+v", env)
	}
}

func TestSubscribeReceivesLiveEvent(t *testing.T) {
	c, _ := liveClient(t)
	ctx, cancel := context.WithTimeout(context.Background(), 5*time.Second)
	defer cancel()

	if err := c.UpsertChannel(ctx, "sdk-go-live", ChannelConfig{ResponseMode: "echo"}); err != nil {
		t.Fatalf("upsert: %v", err)
	}

	got := make(chan Envelope, 1)
	errCh := make(chan error, 1)
	go func() {
		errCh <- c.Subscribe(ctx, "sdk-go-live", "newest", func(env Envelope) {
			// A real subscriber filters its own stream; warmup traffic is ignored.
			if env.Path == "/live" {
				select {
				case got <- env:
				default:
				}
			}
		})
	}()

	// produce only after the first live event confirms the stream is attached:
	// capture an event via Once (handshake-safe), then the streaming Subscribe
	// must also see subsequent events.
	if _, err := c.Once(ctx, "sdk-go-live", "newest", func() error {
		resp, err := c.HTTP.Post(c.CaptureURL("sdk-go-live")+"/warmup", "text/plain",
			strings.NewReader("warmup"))
		if err != nil {
			return err
		}
		return resp.Body.Close()
	}); err != nil {
		t.Fatalf("once warmup: %v", err)
	}

	c.HTTP.Post(c.CaptureURL("sdk-go-live")+"/live", "text/plain", strings.NewReader("ping"))

	select {
	case env := <-got:
		if env.Path != "/live" || env.Body != "ping" {
			t.Fatalf("unexpected event: %+v", env)
		}
	case err := <-errCh:
		t.Fatalf("subscribe ended early: %v", err)
	case <-time.After(4 * time.Second):
		t.Fatal("timed out waiting for live event")
	}
}
