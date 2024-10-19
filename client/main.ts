const relayUrl = "http://localhost:8000"; // Go relay server URL (HTTP server)
const socketUrl = "ws://localhost:9000/events?token=static-token"; // WebSocket URL with token in query parameter

let socket: WebSocket | null = null; // Declare WebSocket connection variable

// Function to initiate an HTTP request to the Go server (no auth required)
async function initiateConnection() {
  try {
    // Make a GET request to the Go relay server without Authorization header
    const response = await fetch(relayUrl, {
      method: "GET",
    });

    if (response.ok) {
      console.log("Successfully connected to Go relay server");

      // Now, initiate the WebSocket connection to the Go WebSocket server
      socket = new WebSocket(socketUrl);

      // WebSocket open event
      socket.onopen = () => {
        console.log("Connected to Go WebSocket server");
        // Now you can send sample events to the Go server
        sendSampleEvents();
      };

      // WebSocket message event (handling messages from Go WebSocket server)
      socket.onmessage = (event) => {
        const data = JSON.parse(event.data);
        console.log("Message received from Go WebSocket server:", data);
      };

      // WebSocket error event
      socket.onerror = (error) => {
        console.error("WebSocket error:", error);
      };

      // WebSocket close event
      socket.onclose = () => {
        console.log("Disconnected from Go WebSocket server");
      };
    } else {
      console.error(
        "Failed to connect to Go relay server:",
        response.statusText
      );
    }
  } catch (error) {
    console.error("Error during connection to Go relay server:", error);
  }
}

// Function to send sample events to the Go WebSocket server
function sendSampleEvents() {
  if (socket && socket.readyState === WebSocket.OPEN) {
    // Example event 1: A sample GET request
    const event1 = {
      method: "GET",
      url: "/api/example1",
      data: { message: "Sample GET request event" },
    };
    socket.send(JSON.stringify(event1));
    console.log("Sent event 1:", event1);

    // Example event 2: A sample POST request
    const event2 = {
      method: "POST",
      url: "/api/example2",
      data: { message: "Sample POST request event" },
    };
    socket.send(JSON.stringify(event2));
    console.log("Sent event 2:", event2);

    // Example event 3: A custom message with random data
    const event3 = {
      method: "CUSTOM",
      url: "/api/custom",
      data: { message: "Sample custom event", timestamp: Date.now() },
    };
    socket.send(JSON.stringify(event3));
    console.log("Sent event 3:", event3);
  } else {
    console.error("WebSocket is not open, cannot send events");
  }
}

// Initiate connection to the Go relay server
initiateConnection();
