// There is a note in the docs that say 'don't put sensitive data in json' and it runs a server. Don't know why. Isn't the point 
// I don't need a server? The React directions don't take the extra step.
const auth_config = {
  domain: "dev-604lx2uiz5wlztvi.us.auth0.com",
  clientId: "vJp0kiTALY9aGmnUltrhPFqZ6e5aQufP"
};

let auth0Client = null;

const login = async () => {
	if (auth0Client) {
		await auth0Client.loginWithPopup(  { authorizationParams: {}});
	}
};

const retrieveJSON = async () => {
	// Get the token
	const accessToken = await auth0Client.getTokenSilently( { detailedResponse: true });
	// Call Netlify with the Token
//	const result = await fetch('https://heart-rate-drift.netlify.app/get-strava-activity', {
	fetch('http://localhost:8888/.netlify/functions/get-strava-activity', {
		method: 'GET',
		headers: {
			'Authorization': `Bearer ${accessToken}`
		}
	}).then((data) => {
			console.log("Data time");
			console.log(data);
			data.text().then((text) => { console.log(`text is ${text}`); });
		});

	// You will need to make this a button with the field eventually
/*	const result = await fetch('https://www.strava.com/api/v3/athlete', {
		method: 'GET',
		headers: {
			'Authorization': auth
		}
	});*/
//	let json = await result.text();
//	console.log(json);
}

addEventListener("TrunkApplicationStarted", async (event) => {
	console.log("application started - bindings:", window.wasmBindings, "WASM:", event.detail.wasm);

	// Start by authenticating
	auth0Client = await auth0.createAuth0Client(auth_config);

	const isAuthenticated = await auth0Client.isAuthenticated();

	console.log(isAuthenticated);


	// THIS WAS STRAIGHT COPIED FROM CHATGPT
	// Since I'd prefer to rewrite in Rust, I'm being very quick and dirty
	const dropArea = document.getElementById('drop-area');

	// Prevent the default behavior of the dragover and drop events (to allow the drop)
	dropArea.addEventListener('dragover', (event) => {
		event.preventDefault();
		dropArea.style.backgroundColor = '#f0f0f0'; // Change color when dragging over
	});

	dropArea.addEventListener('dragleave', () => {
		dropArea.style.backgroundColor = ''; // Revert to the original color
	});

	dropArea.addEventListener('drop', (event) => {
		event.preventDefault(); // Prevent the default behavior (e.g., opening the file)
		dropArea.style.backgroundColor = ''; // Revert to original color

		// Get the dropped files (assuming only one file is dropped)
		const file = event.dataTransfer.files[0];

		if (file && file.type === 'application/json') {
			const reader = new FileReader();

			// Read the file as text
			reader.onload = () => {
				try {
					// Parse the file content as JSON
					const jsonData = JSON.parse(reader.result);
					window.wasmBindings.calculate_heart_rate_drift(
						jsonData.heartrate.data,
						jsonData.time.data
					);

					// You can now use the jsonData in your application
				} catch (error) {
					console.error('Error parsing JSON:', error);
				}
			};

			// Start reading the file
			reader.readAsText(file);
		} else {
			alert('Please drop a valid JSON file.');
		}
	});

});


