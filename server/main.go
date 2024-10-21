package main

import (
	"encoding/json"
	"fmt"
	"io"
	"log"
	"net"
	"net/http"
	"os"
	"strings"
	"sync"
	"time"

	"github.com/gorilla/websocket"
	"github.com/joho/godotenv"
	"gorm.io/driver/postgres"
	"gorm.io/gorm"
	"gorm.io/gorm/logger"
)

var (
	clients      = make(map[string]map[*websocket.Conn]bool) // Map of userID to their clients
	clientsMutex sync.Mutex
	db           *gorm.DB
)

// Map of tokens to user IDs (example tokens)
var userTokens = map[string]string{
	"token1": "user1",
	"token2": "user2",
}

// WebhookEvent represents a webhook event to be stored in the database
type WebhookEvent struct {
	ID        uint `gorm:"primaryKey"`
	UserID    string
	Method    string
	URL       string
	Headers   string
	Body      string
	Timestamp time.Time
}

// Initialize the database connection and migrate the schema
func initDatabase() (*gorm.DB, error) {
	host := os.Getenv("POSTGRES_HOST")
	port := os.Getenv("POSTGRES_PORT")
	user := os.Getenv("POSTGRES_USER")
	password := os.Getenv("POSTGRES_PASSWORD")
	dbname := os.Getenv("POSTGRES_DB")

	dsn := fmt.Sprintf("host=%s user=%s password=%s dbname=%s port=%s sslmode=disable TimeZone=UTC",
		host, user, password, dbname, port)

	db, err := gorm.Open(postgres.Open(dsn), &gorm.Config{
		Logger: logger.Default.LogMode(logger.Info),
	})
	if err != nil {
		return nil, err
	}

	// Migrate the schema
	err = db.AutoMigrate(&WebhookEvent{})
	if err != nil {
		return nil, err
	}

	return db, nil
}

var upgrader = websocket.Upgrader{
	CheckOrigin: func(r *http.Request) bool {
		return true
	},
}

// Start the WebSocket server on port 9000
func startWebSocketServer() {
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

	userID, ok := userTokens[token]
	if !ok {
		http.Error(w, "Unauthorized", http.StatusUnauthorized)
		log.Printf("Unauthorized WebSocket connection attempt from %s", r.RemoteAddr)
		return
	}

	log.Printf("Received WebSocket upgrade request from %s for user %s", r.RemoteAddr, userID)
	conn, err := upgrader.Upgrade(w, r, nil)
	if err != nil {
		log.Printf("Failed to upgrade WebSocket connection: %v", err)
		return
	}
	defer conn.Close()

	// Add the new client to the map
	clientsMutex.Lock()
	if clients[userID] == nil {
		clients[userID] = make(map[*websocket.Conn]bool)
	}
	clients[userID][conn] = true
	clientsMutex.Unlock()

	log.Printf("Client connected: %s (user %s)", conn.RemoteAddr(), userID)

	// Keep the connection open and listen for incoming messages
	for {
		_, message, err := conn.ReadMessage()
		if err != nil {
			log.Printf("Client disconnected or error: %v", err)
			clientsMutex.Lock()
			delete(clients[userID], conn)
			if len(clients[userID]) == 0 {
				delete(clients, userID)
			}
			clientsMutex.Unlock()
			break
		}
		// Handle messages from client if necessary
		// Echo back the message or process as needed
		log.Printf("Received message from client %s (user %s): %s", conn.RemoteAddr(), userID, message)
		// For example, echo the message back
		err = conn.WriteMessage(websocket.TextMessage, message)
		if err != nil {
			log.Printf("Error sending message to client: %v", err)
			continue
		}
	}
}

// Broadcast a message to all connected WebSocket clients for a specific user
func broadcastMessage(userID string, message []byte) {
	clientsMutex.Lock()
	defer clientsMutex.Unlock()

	userClients, exists := clients[userID]
	if !exists {
		log.Printf("No clients connected for user %s", userID)
		return
	}

	for conn := range userClients {
		err := conn.WriteMessage(websocket.TextMessage, message)
		if err != nil {
			log.Printf("Error broadcasting message to client: %v", err)
			conn.Close()
			delete(userClients, conn)
		}
	}

	if len(userClients) == 0 {
		delete(clients, userID)
	}
}

// Handler to retrieve all events for a user (authenticated)
func getUserEvents(w http.ResponseWriter, r *http.Request) {
	token := r.Header.Get("Authorization")
	userID, ok := userTokens[token]
	if !ok {
		http.Error(w, "Unauthorized", http.StatusUnauthorized)
		return
	}

	var events []WebhookEvent
	if err := db.Where("user_id = ?", userID).Find(&events).Error; err != nil {
		http.Error(w, "Failed to retrieve events", http.StatusInternalServerError)
		return
	}

	w.Header().Set("Content-Type", "application/json")
	if err := json.NewEncoder(w).Encode(events); err != nil {
		http.Error(w, "Failed to encode events", http.StatusInternalServerError)
	}
}

// Handler to clear all events for a user (authenticated)
func clearUserEvents(w http.ResponseWriter, r *http.Request) {
	token := r.Header.Get("Authorization")
	userID, ok := userTokens[token]
	if !ok {
		http.Error(w, "Unauthorized", http.StatusUnauthorized)
		return
	}

	if err := db.Where("user_id = ?", userID).Delete(&WebhookEvent{}).Error; err != nil {
		http.Error(w, "Failed to clear events", http.StatusInternalServerError)
		return
	}

	w.Write([]byte("Events cleared"))
}

func main() {
	// Load environment variables from .env file
	err := godotenv.Load()
	if err != nil {
		log.Fatalf("Error loading .env file: %v", err)
	}

	// Initialize the database
	db, err = initDatabase()
	if err != nil {
		log.Fatalf("Failed to connect to database: %v", err)
	}

	log.Println("Starting HTTP server on :8000")

	// Handle incoming HTTP requests without requiring authorization
	http.HandleFunc("/", func(w http.ResponseWriter, r *http.Request) {
		// Extract user ID from subdomain
		host := r.Host
		userID := extractUserIDFromHost(host)
		if userID == "" {
			http.Error(w, "User ID missing in subdomain", http.StatusBadRequest)
			return
		}

		// Optionally validate userID
		validUser := false
		for _, uid := range userTokens {
			if userID == uid {
				validUser = true
				break
			}
		}
		if !validUser {
			http.Error(w, "Invalid user ID", http.StatusBadRequest)
			return
		}

		clientsMutex.Lock()
		activeClients := len(clients[userID])
		clientsMutex.Unlock()

		if activeClients == 0 {
			// No active clients for this user, optionally you can choose to skip processing
			log.Printf("No active clients for user %s", userID)
		}

		// Read the request body
		bodyBytes, err := io.ReadAll(r.Body)
		if err != nil {
			http.Error(w, "Failed to read request body", http.StatusInternalServerError)
			return
		}
		r.Body.Close()

		// Convert headers to a JSON string
		headersJSON, err := json.Marshal(r.Header)
		if err != nil {
			http.Error(w, "Failed to marshal headers", http.StatusInternalServerError)
			return
		}

		// Create a WebhookEvent instance
		event := WebhookEvent{
			UserID:    userID,
			Method:    r.Method,
			URL:       r.URL.String(),
			Headers:   string(headersJSON),
			Body:      string(bodyBytes),
			Timestamp: time.Now(),
		}

		// Save to the database
		if err := db.Create(&event).Error; err != nil {
			http.Error(w, "Failed to save to database", http.StatusInternalServerError)
			return
		}

		// Create a JSON message to broadcast
		message := map[string]interface{}{
			"method": r.Method,
			"url":    r.URL.String(),
			"body":   string(bodyBytes),
		}
		messageBytes, err := json.Marshal(message)
		if err != nil {
			log.Printf("Failed to marshal message: %v", err)
			return
		}

		// Broadcast the message to all WebSocket clients for this user
		broadcastMessage(userID, messageBytes)

		w.Write([]byte("Request logged and forwarded"))
	})

	// Endpoint to retrieve all events for a user (requires authentication)
	http.HandleFunc("/user/events", getUserEvents)

	// Endpoint to clear all events for a user (requires authentication)
	http.HandleFunc("/user/clear", clearUserEvents)

	// Start the WebSocket server in a separate goroutine
	go startWebSocketServer()

	// Start the main HTTP server
	if err := http.ListenAndServe(":8000", nil); err != nil {
		log.Fatalf("HTTP server failed to start: %v", err)
	}
}

// Extract user ID from subdomain
func extractUserIDFromHost(host string) string {
	// Remove port if present
	host = stripPort(host)

	parts := strings.Split(host, ".")
	if len(parts) < 2 {
		return ""
	}
	subdomain := parts[0]
	return subdomain
}

// Helper function to strip port from host if present
func stripPort(host string) string {
	if strings.Contains(host, ":") {
		h, _, err := net.SplitHostPort(host)
		if err == nil {
			return h
		}
	}
	return host
}
