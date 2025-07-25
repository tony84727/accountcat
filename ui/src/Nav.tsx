import type { Response } from "./GoogleSignIn";
import GoogleSignIn from "./GoogleSignIn";
import styles from "./Nav.module.scss";

interface Props {
	username?: string;
	promptLogin?: boolean;
	onLogin(response: Response): void;
}

export default function Nav({ username, promptLogin, onLogin }: Props) {
	return (
		<nav className={styles.container}>
			AccountCat
			{username && <span>Hello, {username}</span>}
			{promptLogin && <GoogleSignIn loginCallback={onLogin} />}
		</nav>
	);
}
