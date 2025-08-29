export default function formatInputNumber(input: string): string {
	const sanitized = input.replace(/[^\d.]/g, "");
	const matches = sanitized.match(/\d*(?:\.\d*)?/);
	if (!matches || matches[0] === "") {
		return "0";
	}
	const result = matches[0];
	if (result.startsWith(".")) {
		return `0${result}`;
	}
	return result.replace(/^0+(?=\d)/, "");
}
