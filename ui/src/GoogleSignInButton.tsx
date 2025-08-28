import { useContext, useEffect, useRef } from "react";
import GsiContext from "./GsiContext";

export default function GoogleSignInButton() {
	const buttonRef = useRef(null);
	const gsi = useContext(GsiContext);
	useEffect(() => {
		if (!buttonRef.current) {
			return;
		}
		if (!gsi.loaded) {
			gsi.load?.();
			return;
		}
		const element = document.querySelector(".g_id_signin") as HTMLElement;
		if (!element) {
			return;
		}
		google.accounts.id.renderButton(element, {
			type: "standard",
			theme: "filled_blue",
		});
	}, [gsi]);
	return (
		<div
			ref={buttonRef}
			className="g_id_signin"
			data-type="standard"
			data-shape="rectangular"
			data-theme="outline"
			data-text="signin_with"
			data-size="large"
			data-logo_alignment="left"
		></div>
	);
}
