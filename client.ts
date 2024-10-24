// test-client.js
const users = [
  { username: "alice", subdomain: "alice" },
  { username: "bob", subdomain: "bob" },
  { username: "charlie", subdomain: "charlie" },
  { username: "dave", subdomain: "dave" },
];

// Function to create WebSocket connection for each user
function createWebSocketConnection(subdomain: string) {
  const ws = new WebSocket(`ws://${subdomain}.localhost:3000`);

  ws.onopen = () => {
    console.log(`Connected to WebSocket server as ${subdomain}.`);
    ws.send(
      JSON.stringify({ event: "hello", message: `Hello from ${subdomain}!` })
    );
  };

  ws.onmessage = (event) => {
    console.log(`Message from server for ${subdomain}:`, event.data);
  };

  ws.onerror = (error) => {
    console.error(`WebSocket error for ${subdomain}:`, error);
  };

  ws.onclose = () => {
    console.log(`WebSocket connection closed for ${subdomain}.`);
  };

  return ws;
}

// Function to send HTTP events
async function sendHttpEvent(user: { username: string; subdomain: string }) {
  const url = `http://${user.subdomain}.localhost:3000`;

  const response = await fetch(url, {
    method: "POST",
    headers: {
      "Content-Type": "application/json",
    },
    body: JSON.stringify({
      user: user.username,
      message: `Event from ${user.username}`,
    }),
  });

  const responseBody = await response.text();
  console.log(`HTTP response for ${user.username}:`, responseBody);
}

// Create WebSocket connections and send HTTP events
async function main() {
  const websockets = users.map((user) =>
    createWebSocketConnection(user.subdomain)
  );

  // Allow some time for WebSocket connections to open before sending HTTP requests
  await new Promise((resolve) => setTimeout(resolve, 2000));

  for (const user of users) {
    await sendHttpEvent(user);
  }

  // Optionally close the connections after sending events
  websockets.forEach((ws) => ws.close());
}

setTimeout(main, 1000);
