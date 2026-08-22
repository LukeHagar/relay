// WebSocket subscriptions for the Go SDK (docs/api.md §5.1).
package relay

import (
	"context"
	"encoding/json"
	"fmt"
	"net/http"
	"strconv"
	"strings"

	"github.com/gorilla/websocket"
)

// Subscribe connects to /ws, subscribes to channel starting at from
// ("newest" | "oldest" | decimal sequence number), and invokes handler for every
// event until ctx is canceled or the socket closes. Backlog is delivered first,
// ascending, with no gap or duplicate at the live seam (§5.1).
func (c *Client) dialAndSubscribe(ctx context.Context, channel, from, corrID string) (*websocket.Conn, error) {
	wsURL := wsScheme(c.BaseURL) + "/ws"
	dialer := websocket.Dialer{}
	requestHeader := http.Header{}
	if c.Token != "" {
		requestHeader.Set("Authorization", "Bearer "+c.Token)
	}
	conn, _, err := dialer.DialContext(ctx, wsURL, requestHeader)
	if err != nil {
		return nil, err
	}
	sub, err := json.Marshal(map[string]any{
		"type":    "subscribe",
		"id":      corrID,
		"channel": channel,
		"from":    parseFrom(from),
	})
	if err != nil {
		conn.Close()
		return nil, err
	}
	if err := conn.WriteMessage(websocket.TextMessage, sub); err != nil {
		conn.Close()
		return nil, err
	}
	return conn, nil
}

// Subscribe streams every event on the channel to handler until ctx is done or the
// socket closes. See Once for a one-shot, handshake-safe variant.
func (c *Client) Subscribe(ctx context.Context, channel, from string, handler func(Envelope)) error {
	conn, err := c.dialAndSubscribe(ctx, channel, from, "go-"+channel)
	if err != nil {
		return err
	}
	defer conn.Close()

	for {
		select {
		case <-ctx.Done():
			return context.Cause(ctx)
		default:
		}
		_, raw, err := conn.ReadMessage()
		if err != nil {
			return err
		}
		var frame struct {
			Type  string `json:"type"`
			Error *struct {
				Code    string `json:"code"`
				Message string `json:"message"`
			} `json:"error"`
			Event   *Envelope `json:"event"`
			Channel string    `json:"channel"`
		}
		if json.Unmarshal(raw, &frame) != nil {
			continue
		}
		switch frame.Type {
		case "event":
			if frame.Event != nil {
				handler(*frame.Event)
			}
		case "error":
			code, msg := "unknown", ""
			if frame.Error != nil {
				code, msg = frame.Error.Code, frame.Error.Message
			}
			return fmt.Errorf("relay: subscription error %s: %s", code, msg)
		case "channel_closed":
			return fmt.Errorf("relay: channel %q closed", frame.Channel)
		}
	}
}

// Once awaits exactly one live event on the channel. When produce is given it runs
// only after the engine confirms the subscription ("ok" frame), so the produced
// delivery can never be missed.
func (c *Client) Once(ctx context.Context, channel, from string, produce func() error) (Envelope, error) {
	conn, err := c.dialAndSubscribe(ctx, channel, from, "go-once-"+channel)
	if err != nil {
		return Envelope{}, err
	}
	defer conn.Close()

	var produced bool
	for {
		select {
		case <-ctx.Done():
			return Envelope{}, context.Cause(ctx)
		default:
		}
		_, raw, err := conn.ReadMessage()
		if err != nil {
			return Envelope{}, err
		}
		var frame struct {
			Type  string `json:"type"`
			Error *struct {
				Code    string `json:"code"`
				Message string `json:"message"`
			} `json:"error"`
			Event *Envelope `json:"event"`
		}
		if json.Unmarshal(raw, &frame) != nil {
			continue
		}
		switch frame.Type {
		case "ok":
			if !produced && produce != nil {
				produced = true
				if err := produce(); err != nil {
					return Envelope{}, err
				}
			}
		case "event":
			if frame.Event != nil {
				return *frame.Event, nil
			}
		case "error":
			code, msg := "unknown", ""
			if frame.Error != nil {
				code, msg = frame.Error.Code, frame.Error.Message
			}
			return Envelope{}, fmt.Errorf("relay: subscription error %s: %s", code, msg)
		}
	}
}

func parseFrom(s string) any {
	if n, err := strconv.ParseUint(s, 10, 64); err == nil {
		return n
	}
	return s // "newest" | "oldest"; engine rejects anything else
}

func wsScheme(base string) string {
	base = strings.TrimRight(base, "/")
	switch {
	case strings.HasPrefix(base, "https://"):
		return "wss://" + strings.TrimPrefix(base, "https://")
	case strings.HasPrefix(base, "http://"):
		return "ws://" + strings.TrimPrefix(base, "http://")
	default:
		return base
	}
}
