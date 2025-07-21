import styles from "./Nav.module.scss";

interface Props {
	onLogin?(): void;
}

export default function (props: Props) {
	return (
		<nav className={styles.container}>
			AccountCat
			<button onClick={props.onLogin} type="button">
				Login
			</button>
		</nav>
	);
}
