import styles from "./Nav.module.scss";

interface Props {
	username?: string;
}

export default function Nav(props: Props) {
	return (
		<nav className={styles.container}>
			AccountCat
			{props.username && <span>Hello, {props.username}</span>}
		</nav>
	);
}
