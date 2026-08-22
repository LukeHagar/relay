// Package relay is the Go SDK for the Relay engine — a thin, typed client over the
// wire contract in docs/api.md (control plane + capture plane + WebSocket
// subscriptions).
//
// Zero dependencies beyond gorilla/websocket.
package relay

import (
	"bytes"
	"context"
	"encoding/json"
	"fmt"
	"io"
	"net/http"
	"strings"
	"time"
)

// Envelope mirrors docs/api.md §4 exactly.
type Envelope struct {
	V            int         `json:"v"`
	ID           string      `json:"id"`
	Channel      string      `json:"channel"`
	Host         *string     `json:"host"`
	Seq          uint64      `json:"seq"`
	ReceivedAt   string      `json:"received_at"`
	Method       string      `json:"method"`
	Path         string      `json:"path"`
	Query        *string     `json:"query"`
	Headers      [][2]string `json:"headers"`
	Body         string      `json:"body"`
	BodyEncoding string      `json:"body_encoding"` // "utf8" | "base64"
	Truncated    bool        `json:"truncated"`
	StatusSent   *int        `json:"status_sent"`
	RemoteAddr   *string     `json:"remote_addr"`
}

// ForwardRule re-delivers every captured event to an additional target.
type ForwardRule struct {
	URL     string      `json:"url"`
	Profile string      `json:"profile,omitempty"`
	Headers [][2]string `json:"headers,omitempty"`
}

// ChannelConfig is the mutable channel descriptor.
type ChannelConfig struct {
	ResponseMode string `json:"response_mode,omitempty"` // "ack" | "echo" | "static"
	StaticResponse *struct {
		Status  int         `json:"status"`
		Headers [][2]string `json:"headers,omitempty"`
		Body    string      `json:"body,omitempty"`
	} `json:"static_response,omitempty"`
	Auth *struct {
		BearerEnv string `json:"bearer_env"`
	} `json:"auth,omitempty"`
	Buffer *struct {
		MaxEvents int `json:"max_events,omitempty"`
		MaxAgeS   int `json:"max_age_s,omitempty"`
	} `json:"buffer,omitempty"`
	RateLimit *struct {
		Burst      uint32 `json:"burst"`
		RefillPerS uint32 `json:"refill_per_s"`
	} `json:"rate_limit,omitempty"`
	Forward []ForwardRule `json:"forward,omitempty"`
}

// InlineSource is a fully specified outgoing request.
type InlineSource struct {
	Method  string      `json:"method,omitempty"`
	Path    string      `json:"path,omitempty"`
	Query   string      `json:"query,omitempty"`
	Headers [][2]string `json:"headers,omitempty"`
	Body    string      `json:"body,omitempty"`
}

// ReplaySource selects a stored event, a template, or an inline request.
type ReplaySource struct {
	EventID  *string       `json:"event_id,omitempty"`
	Template *string       `json:"template,omitempty"`
	Inline   *InlineSource `json:"inline,omitempty"`
}

// Overrides adjust the outgoing request after the source.
type Overrides struct {
	Method  string      `json:"method,omitempty"`
	Path    string      `json:"path,omitempty"`
	Query   *string     `json:"query,omitempty"`
	Headers [][2]string `json:"headers,omitempty"`
	Body    *string     `json:"body,omitempty"`
}

// SigningSpec selects a named profile (secrets resolve engine-side) or an inline
// scheme+secret pair (requires --allow-inline-secret on the engine).
type SigningSpec struct {
	Profile string `json:"profile,omitempty"`
	Scheme  string `json:"scheme,omitempty"`
	Secret  string `json:"secret,omitempty"`
}

// ReplayRequest mirrors docs/api.md §6.
type ReplayRequest struct {
	TargetURL string       `json:"target_url"`
	TimeoutMs int          `json:"timeout_ms,omitempty"`
	Source    ReplaySource `json:"source"`
	Overrides *Overrides   `json:"overrides,omitempty"`
	Vars      map[string]any `json:"vars,omitempty"`
	Signing   *SigningSpec `json:"signing,omitempty"`
}

// Delivery reports the real outcome of the replayed delivery.
type Delivery struct {
	Status     *int    `json:"status"`
	DurationMs int64   `json:"duration_ms"`
	Error      *string `json:"error"`
}

// ReplayResult echoes exactly what went out plus the delivery outcome.
type ReplayResult struct {
	SentEvent Envelope `json:"sent_event"`
	Delivery  Delivery `json:"delivery"`
}

// Error is the typed non-2xx payload: {"error":{"code","message"}}.
type Error struct {
	Status int
	Code   string
	Body   string
}

func (e *Error) Error() string {
	return fmt.Sprintf("relay: %s (%d): %s", e.Code, e.Status, e.Body)
}

// Client talks to one engine's control plane. It is safe for concurrent use.
type Client struct {
	BaseURL     string // control plane origin, e.g. http://127.0.0.1:8000
	CaptureBase string // capture plane origin, e.g. http://127.0.0.1:8001
	Token       string
	HTTP        *http.Client
}

// New returns a client with sane defaults (30s HTTP timeout).
func New(baseURL, token string) *Client {
	return &Client{
		BaseURL:     strings.TrimRight(baseURL, "/"),
		CaptureBase: "http://127.0.0.1:8001",
		Token:       token,
		HTTP:        &http.Client{Timeout: 30 * time.Second},
	}
}

func (c *Client) do(ctx context.Context, method, path string, body any, out any) error {
	var reader io.Reader
	if body != nil {
		raw, err := json.Marshal(body)
		if err != nil {
			return err
		}
		reader = bytes.NewReader(raw)
	}
	req, err := http.NewRequestWithContext(ctx, method, c.BaseURL+path, reader)
	if err != nil {
		return err
	}
	if body != nil {
		req.Header.Set("content-type", "application/json")
	}
	if c.Token != "" {
		req.Header.Set("authorization", "Bearer "+c.Token)
	}
	resp, err := c.HTTP.Do(req)
	if err != nil {
		return err
	}
	defer resp.Body.Close()
	raw, _ := io.ReadAll(resp.Body)
	if resp.StatusCode >= 300 {
		code := fmt.Sprintf("http_%d", resp.StatusCode)
		var typed struct {
			Error struct {
				Code    string `json:"code"`
				Message string `json:"message"`
			} `json:"error"`
		}
		if json.Unmarshal(raw, &typed) == nil && typed.Error.Code != "" {
			code = typed.Error.Code
		}
		return &Error{Status: resp.StatusCode, Code: code, Body: string(raw)}
	}
	if out != nil && len(raw) > 0 {
		return json.Unmarshal(raw, out)
	}
	return nil
}

// UpsertChannel creates or updates a channel (201 on create, 200 on update — both OK).
func (c *Client) UpsertChannel(ctx context.Context, name string, cfg ChannelConfig) error {
	return c.do(ctx, http.MethodPut, "/api/channels/"+name, cfg, nil)
}

// DeleteChannel drops the channel, its buffer and subscriptions.
func (c *Client) DeleteChannel(ctx context.Context, name string) error {
	return c.do(ctx, http.MethodDelete, "/api/channels/"+name, nil, nil)
}

// History returns events ascending, strictly above cursor (0 = from oldest).
func (c *Client) History(ctx context.Context, name string, cursor uint64, limit int) ([]Envelope, error) {
	path := fmt.Sprintf("/api/channels/%s/events?cursor=%d&limit=%d", name, cursor, limit)
	var page struct {
		Events []Envelope `json:"events"`
	}
	if err := c.do(ctx, http.MethodGet, path, nil, &page); err != nil {
		return nil, err
	}
	return page.Events, nil
}

func httpNewRequest(method, url, body string) (*http.Request, error) {
	return http.NewRequestWithContext(context.Background(), method, url, strings.NewReader(body))
}

func strPtr(s string) *string { return &s }

func asRelayError(err error, target **Error) bool {
	e, ok := err.(*Error)
	if ok {
		*target = e
	}
	return ok
}

// CaptureURL is the provider-facing URL for the channel's capture root.
func (c *Client) CaptureURL(channel string) string {
	base := c.CaptureBase
	if base == "" {
		base = "http://127.0.0.1:8001"
	}
	return strings.TrimRight(base, "/") + "/c/" + channel
}

// Replay sends a stored/template/inline event and reports the real delivery outcome.
func (c *Client) Replay(ctx context.Context, req ReplayRequest) (*ReplayResult, error) {
	var result ReplayResult
	if err := c.do(ctx, http.MethodPost, "/api/replay", req, &result); err != nil {
		return nil, err
	}
	return &result, nil
}
