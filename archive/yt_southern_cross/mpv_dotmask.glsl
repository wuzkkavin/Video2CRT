//!HOOK RGB
//!BIND HOOKED
//!DESC CRT Phosphor RGB dotmask (vertical stripes, 3px period) + horizontal scanline

vec4 hook()
{
    vec4 c = HOOKED_tex(HOOKED_pos);

    // RGB phosphor dotmask (vertical stripes, 3px period)
    float x = HOOKED_pos.x * HOOKED_size.x;
    int col = int(mod(x, 3.0));
    float modR = (col == 0) ? 1.00 : 0.92;
    float modG = (col == 1) ? 1.00 : 0.92;
    float modB = (col == 2) ? 1.00 : 0.92;

    // Horizontal scanline (4px period, 2px scanline per 4px)
    float y = HOOKED_pos.y * HOOKED_size.y;
    int row = int(mod(y, 4.0));
    float vfade = (row < 2) ? 1.00 : 0.86;

    return vec4(c.r * modR * vfade, c.g * modG * vfade, c.b * modB * vfade, c.a);
}