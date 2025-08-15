import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App";

const nonce_tag = document.querySelector("meta[name='nonce']");
const nonce = nonce_tag?.getAttribute("content");
if (nonce) {
	window.__webpack_nonce__ = nonce;
}
const rootEl = document.getElementById("root");
if (rootEl) {
	const root = ReactDOM.createRoot(rootEl);
	root.render(
		<React.StrictMode>
			<App />
		</React.StrictMode>,
	);
}
