/* Evaluates one NRLMSISE-00 case, read from standard input, with the
 * reference C implementation. argv[1] is "gtd7" or "gtd7d". */
#include <stdio.h>
#include <string.h>
#include "nrlmsise-00.h"

int main(int argc, char **argv) {
	struct nrlmsise_output output;
	struct nrlmsise_input input;
	struct nrlmsise_flags flags;
	struct ap_array aph;
	int hist, i, sw[23];
	if (argc != 2) return 2;
	input.year = 0;
	if (scanf("%d %lf %lf %lf %lf %lf %lf %lf %d", &input.doy, &input.sec, &input.alt, &input.g_lat, &input.g_long, &input.lst, &input.f107A, &input.f107, &hist) != 9) return 3;
	for (i = 0; i < 7; i++) if (scanf("%lf", &aph.a[i]) != 1) return 3;
	for (i = 0; i < 23; i++) if (scanf("%d", &sw[i]) != 1) return 3;
	input.ap = aph.a[0];
	input.ap_a = &aph;
	flags.switches[0] = 0;
	for (i = 0; i < 23; i++) flags.switches[i + 1] = sw[i];
	/* The Fortran TSELEC reduces switch 9 modulo 2; the C port does not. */
	if (flags.switches[9] == 2) flags.switches[9] = 0;
	if (hist && flags.switches[9] == 1) flags.switches[9] = -1;
	if (strcmp(argv[1], "gtd7d") == 0) gtd7d(&input, &flags, &output);
	else gtd7(&input, &flags, &output);
	for (i = 0; i < 9; i++) printf("%.16e ", output.d[i]);
	printf("%.16e %.16e\n", output.t[0], output.t[1]);
	return 0;
}
