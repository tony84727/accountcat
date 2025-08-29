export default function formatInputNumber(input: string): string {
	const matches = input.replace(/[^\d.]/g, "").match(/\d+.?(\d+)?/);
	if (!matches) {
		return "0";
	}
	return matches[0].replace(/^0+(?=[^0]+)/, "");
}
