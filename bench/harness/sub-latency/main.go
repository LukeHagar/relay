// sub-latency measures CAPTURE→SUBSCRIBER delivery latency through a running
// relay engine, dogfooding the repo's own Go SDK (github.com/relay-webhook/relay-sdk-go).
//
// A producer goroutine POSTs timestamped envelopes to the channel's capture
// plane URL at a fixed rate while N SDK subscribers (WebSocket) record, for
// every delivered envelope, latency = time.Now().UnixNano() - t_sent.
//
// Output: exactly one JSON line on stdout:
//
//	{"subscribers":N,"duration_s":X,"produced":P,"samples":S,
//	 "latency_ms":{"p50":..,"p90":..,"p99":..,"max":..},
//	 "dropped_estimate":P*N-S}
//
// Requires the engine already running and the channel pre-created.
package main

import (
	"context"
	"encoding/json"
	"flag"
	"math"
	"fmt"
	"io"
	"log"
	"net/http"
	"os"
	"sort"
	"strings"
	"sync"
	"time"

	relay "github.com/relay-webhook/relay-sdk-go"
)

var (
	control    = flag.String("control", "http://127.0.0.1:8000", "control plane URL")
	capture    = flag.String("capture", "http://127.0.0.1:8001", "capture plane URL")
	channel    = flag.String("channel", "bench-capture", "channel name")
	subscribers = flag.Int("subscribers", 4, "WebSocket subscriber count")
	rate       = flag.Int("rate", 200, "events per second")
	duration   = flag.Duration("duration", 30*time.Second, "run duration")
	mode       = flag.String("mode", "sub-latency", "sub-latency | capture-rps")
	token      = flag.String("token", "", "bearer token")
)

func main() {
	flag.Parse()
	switch *mode {
	case "capture-rps":
		runCaptureRPS()
	default:
		runSubLatency()
	}
}

// capture-rps: saturating HTTP probe against the capture plane. N workers loop
// POSTing unique bodies as fast as they can (bounded by -rate when > 0), recording
// client-observed RTT per request. Prints one JSON line with percentiles and the
// achieved throughput — unbiased by Node/Artillery event-loop lag.
func runCaptureRPS() {
	client := &http.Client{
		Timeout: 10 * time.Second,
		Transport: &http.Transport{
			MaxIdleConns:        512,
			MaxIdleConnsPerHost: 512,
			MaxConnsPerHost:     0,
		},
	}
	url := fmt.Sprintf("%s/c/%s/rps", *capture, *channel)
	workers := 8
	if *rate > 0 && *rate < 2000 {
		workers = 4
	}

	var (
		mu      sync.Mutex
		latency []float64
		count   int
		errs    int
	)
	var wg sync.WaitGroup
	stop := make(chan struct{})
	perWorkerRate := 0
	if *rate > 0 {
		perWorkerRate = *rate / workers
	}

	for w := 0; w < workers; w++ {
		wg.Add(1)
		go func(id int) {
			defer wg.Done()
			seq := 0
			interval := time.Duration(0)
			if perWorkerRate > 0 {
				interval = time.Second / time.Duration(perWorkerRate)
			}
			var next time.Time
			if interval > 0 {
				next = time.Now()
			}
			for {
				select {
				case <-stop:
					return
				default:
				}
				if interval > 0 {
					if d := time.Until(next); d > 0 {
						time.Sleep(d)
					}
					next = next.Add(interval)
				}
				seq++
				body := fmt.Sprintf(`{"worker":%d,"seq":%d,"t_sent":%d}`, id, seq, time.Now().UnixNano())
				start := time.Now()
				req, err := http.NewRequestWithContext(context.Background(), http.MethodPost, url, strings.NewReader(body))
				if err != nil {
					mu.Lock(); errs++; mu.Unlock()
					continue
				}
				req.Header.Set("content-type", "application/json")
				if *token != "" {
					req.Header.Set("authorization", "Bearer "+*token)
				}
				resp, err := client.Do(req)
				if err != nil {
					mu.Lock(); errs++; mu.Unlock()
					continue
				}
				io.Copy(io.Discard, resp.Body)
				resp.Body.Close()
				lat := float64(time.Since(start).Nanoseconds()) / 1e6
				mu.Lock()
				count++
				if resp.StatusCode >= 400 {
					errs++
				} else {
					latency = append(latency, lat)
				}
				mu.Unlock()
			}
		}(w)
	}

	time.Sleep(*duration)
	close(stop)
	wg.Wait()

	sort.Float64s(latency)
	pct := func(p float64) float64 {
		if len(latency) == 0 {
			return 0
		}
		idx := int(math.Round(p * float64(len(latency)-1)))
		return latency[idx]
	}
	achieved := float64(len(latency)) / duration.Seconds()
	out, _ := json.MarshalIndent(map[string]any{
		"mode":        "capture-rps",
		"target_rps":  *rate,
		"duration_s":  duration.Seconds(),
		"workers":     workers,
		"requests":    len(latency),
		"errors":      errs,
		"achieved_rps": math.Round(achieved),
		"latency_ms": map[string]float64{
			"p50": pct(0.50), "p90": pct(0.90), "p95": pct(0.95),
			"p99": pct(0.99), "p999": pct(0.999), "max": latency[len(latency)-1:][0],
		},
	}, "", "  ")
	fmt.Println(string(out))
}

func runSubLatency() {
	if *subscribers < 1 || *rate < 1 {
		log.Fatal("sub-latency: -subscribers and -rate must be >= 1")
	}

	ctx, cancel := context.WithTimeout(context.Background(), *duration+10*time.Second)
	defer cancel()

	client := relay.New(*control, *token)

	// Subscribers first: connect all sockets before the first capture so the
	// producer never races an unconnected consumer.
	var subWG sync.WaitGroup
	var mu sync.Mutex // guards samples
	var samples []int64
	for i := 0; i < *subscribers; i++ {
		subWG.Add(1)
		go func() {
			defer subWG.Done()
			err := client.Subscribe(ctx, *channel, "newest", func(env relay.Envelope) {
				var body struct {
					TSent int64 `json:"t_sent"`
				}
				if json.Unmarshal([]byte(env.Body), &body) != nil || body.TSent == 0 {
					return
				}
				mu.Lock()
				samples = append(samples, time.Now().UnixNano()-body.TSent)
				mu.Unlock()
			})
			if err != nil && ctx.Err() == nil {
				log.Printf("sub-latency: subscriber exited early: %v", err)
			}
		}()
	}

	// Give the WS handshake + subscribe frame a moment before producing.
	time.Sleep(500 * time.Millisecond)

	produced := produce(ctx, *capture, *channel, *rate, *duration)
	cancel() // end the measurement window; subscribers unwind via ctx
	subWG.Wait()

	mu.Lock()
	defer mu.Unlock()
	sort.Slice(samples, func(i, j int) bool { return samples[i] < samples[j] })

	out := map[string]any{
		"subscribers":      *subscribers,
		"duration_s":       duration.Seconds(),
		"produced":         produced,
		"samples":          len(samples),
		"latency_ms":       latencyMS(samples),
		"dropped_estimate": produced*(*subscribers) - len(samples),
	}
	if err := json.NewEncoder(os.Stdout).Encode(out); err != nil {
		log.Fatal(err)
	}
}

// produce POSTs one timestamped envelope per tick until the deadline and
// returns the number of captures accepted by the engine (2xx).
func produce(ctx context.Context, captureBase, channel string, rate int, dur time.Duration) int {
	url := fmt.Sprintf("%s/c/%s/%d", strings.TrimSuffix(captureBase, "/"), channel, time.Now().UnixNano())
	hc := &http.Client{Timeout: 5 * time.Second}
	interval := time.Duration(int64(time.Second) / int64(rate))

	var produced int
	var mu sync.Mutex
	var wg sync.WaitGroup
	ticker := time.NewTicker(interval)
	defer ticker.Stop()
	// `seq` lives on this goroutine only; `produced` is mutex-guarded because
	// POST goroutines bump it from other goroutines.
	seq := 0
	deadline := time.After(dur)

	for {
		select {
		case <-deadline:
			wg.Wait()
			return produced
		case <-ticker.C:
		case <-ctx.Done():
			wg.Wait()
			return produced
		}

		body := fmt.Sprintf(`{"sub":"all","seq":%d,"t_sent":%d}`, seq, time.Now().UnixNano())
		seq++
		wg.Add(1)
		go func() {
			defer wg.Done()
			resp, err := hc.Post(url, "application/json", strings.NewReader(body))
			if err != nil {
				log.Printf("sub-latency: capture POST failed: %v", err)
				return
			}
			io.Copy(io.Discard, resp.Body)
			resp.Body.Close()
			if resp.StatusCode >= 200 && resp.StatusCode < 300 {
				mu.Lock()
				produced++
				mu.Unlock()
			} else {
				log.Printf("sub-latency: capture POST status %d", resp.StatusCode)
			}
		}()
	}
}

// latencyMS returns p50/p90/p99/max in milliseconds from sorted nanosecond
// samples (nearest-rank). Empty input yields zeroed percentiles.
func latencyMS(sorted []int64) map[string]float64 {
	pick := func(p float64) float64 {
		if len(sorted) == 0 {
			return 0
		}
		idx := int(p/100*float64(len(sorted)) + 0.999999)
		if idx < 1 {
			idx = 1
		}
		if idx > len(sorted) {
			idx = len(sorted)
		}
		return float64(sorted[idx-1]) / 1e6
	}
	maxNS := int64(0)
	if len(sorted) > 0 {
		maxNS = sorted[len(sorted)-1]
	}
	return map[string]float64{
		"p50": pick(50),
		"p90": pick(90),
		"p99": pick(99),
		"max": float64(maxNS) / 1e6,
	}
}
