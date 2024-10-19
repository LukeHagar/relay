package main

import (
	"log"
	"net/http"
	"sync"

	"github.com/danielgtaylor/huma/v2/humacli"
	"github.com/gorilla/websocket"
)

const (
	authToken = "static-token" // Replace with a secure token in production
)

var upgrader = websocket.Upgrader{
	CheckOrigin: func(r *http.Request) bool {
		return true
	},
}

var (
	clients      = make(map[*websocket.Conn]bool)
	clientsMutex sync.Mutex
)

// Start the WebSocket HTTP server on port 9000
func startHTTPServer() {
	http.HandleFunc("/events", handleWebSocket)
	log.Println("Starting WebSocket server on :9000")
	err := http.ListenAndServe(":9000", nil)
	if err != nil {
		log.Fatalf("WebSocket server failed to start: %v", err)
	}
}

// Handle WebSocket connections with token-based authorization
func handleWebSocket(w http.ResponseWriter, r *http.Request) {
	token := r.Header.Get("Authorization") // Retrieve token from headers
	if token == "" {
		token = r.URL.Query().Get("token") // Optionally, check query parameter
	}

	if token != authToken {
		http.Error(w, "Unauthorized", http.StatusUnauthorized)
		log.Printf("Unauthorized WebSocket connection attempt from %s", r.RemoteAddr)
		return
	}

	log.Printf("Received WebSocket upgrade request from %s", r.RemoteAddr)
	conn, err := upgrader.Upgrade(w, r, nil)
	if err != nil {
		log.Printf("Failed to upgrade WebSocket connection: %v", err)
		return
	}
	defer conn.Close()

	// Add the new client to the map
	clientsMutex.Lock()
	clients[conn] = true
	clientsMutex.Unlock()

	log.Println("Client connected:", conn.RemoteAddr())

	// Keep the connection open and listen for incoming messages
	for {
		_, message, err := conn.ReadMessage()
		if err != nil {
			log.Printf("Client disconnected or error: %v", err)
			clientsMutex.Lock()
			delete(clients, conn)
			clientsMutex.Unlock()
			break
		}

		// Log and broadcast the received message to all clients
		log.Printf("Received message from client: %s", message)
		broadcastMessage(message)
	}
}

// Broadcast a message to all connected WebSocket clients
func broadcastMessage(message []byte) {
	clientsMutex.Lock()
	defer clientsMutex.Unlock()

	for conn := range clients {
		err := conn.WriteMessage(websocket.TextMessage, message)
		if err != nil {
			log.Printf("Error broadcasting message to client: %v", err)
			conn.Close()
			delete(clients, conn)
		}
	}
}

func main() {
	cli := humacli.New(func(hooks humacli.Hooks, options *struct{}) {
		hooks.OnStart(func() {
			log.Println("Starting Huma relay server on :8000")

			// Handle incoming HTTP requests without requiring authorization
			http.HandleFunc("/", func(w http.ResponseWriter, r *http.Request) {
				log.Printf("Received request from %s: Method: %s, URL: %s", r.RemoteAddr, r.Method, r.URL)

				// Create a JSON message from the HTTP request and broadcast it
				message := map[string]interface{}{
					"method": r.Method,
					"url":    r.URL.String(),
				}

				clientsMutex.Lock()
				for conn := range clients {
					err := conn.WriteJSON(message)
					if err != nil {
						log.Printf("Error sending message to WebSocket client: %v", err)
						conn.Close()
						delete(clients, conn)
					}
				}
				clientsMutex.Unlock()

				w.Write([]byte("Request logged and forwarded"))
			})

			// Start the WebSocket server in a separate goroutine
			go startHTTPServer()

			// Now keep the Huma server running by listening on port 8000
			err := http.ListenAndServe(":8000", nil)
			if err != nil {
				log.Fatalf("Huma relay server failed to start: %v", err)
			}
		})
	})

	cli.Run()
}
