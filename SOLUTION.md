# Solution

Original Code:  
Operated in a blocking manner, processing one client at a time before moving on. This design severely limited scalability.

New Code:  
Implements concurrency by using a thread pool to handle multiple client connections.  
Each incoming connection is assigned to a worker thread from the thread pool, avoiding the overhead of spawning a new thread for every connection.  
Ensures efficient resource usage and improved scalability.

Original Code:  
Worked only with EchoMessage and lacked flexibility to extend for additional message types.

New Code:  
Introduces ClientMessage and ServerMessage structures to support multiple message types.  
Adds the following message types:  
- EchoMessage: Echoes back the received content.  
- AddRequest: Computes the sum of two integers and responds with an AddResponse.  
This modular approach makes the server extensible for future features.

Original Code:  
Read and processed a single message before closing the connection, requiring clients to reconnect for subsequent interactions.

New Code:  
Continuously reads and processes messages in a loop until the client disconnects or an error occurs.  
This enables multiple interactions over a single connection.

Original Code:  
Relied on blocking I/O, risking indefinite hangs when waiting for input.

New Code:  
Implements non-blocking I/O with error handling for ErrorKind::WouldBlock.  
Introduces a brief delay (thread::sleep) before retrying, preventing busy-wait loops.

I added three test cases:  
1. Handling Negative Numbers: A test case was added to ensure the server can correctly handle the addition of negative numbers.  
2. High Traffic Handling: A test case was added to simulate sending a large number of messages to the server.  
3. Post-Disconnection Message Handling: Another test was introduced to verify the server's behavior when attempting to send messages after a client has disconnected.

To address potential issues in test case execution, I added the serial_test crate. In the original implementation, all test cases would run simultaneously, which created conflicts because each test tried to use the server on the same port—a shared resource. To prevent these conflicts, I used the #[serial] attribute in the test cases. This ensures that tests are executed one at a time, allowing the server to be accessed in a controlled manner without interference between tests.
