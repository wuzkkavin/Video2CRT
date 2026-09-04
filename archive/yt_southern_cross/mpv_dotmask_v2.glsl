//!HOOK RGB
//!BIND HOOKED
//!DESC CRT Phosphor Dotmask + Scanlines (rendered at final 1920x1080)

vec4 hook()
{
    vec4 c = HOOKED_tex(HOOKED_pos);

    // RGB phosphor dotmask (vertical stripes, 3px period)
    float x = HOOKED_pos.x * HOOKED_size.x;
    int col = int(mod(x, 3.0));
    float modR = (col == 0) ? 1.00 : 0.90;
    float modG = (col == 1) ? 1.00 : 0.90;
    float modB = (col == 2) ? 1.00 : 0.90;

    // Horizontal scanline (4px period, dark every other row)
    float y = HOOKED_pos.y * HOOKED_size.y;
    int row = int(mod(y, 4.0));
    float vfade = (row < 2) ? 1.00 : 0.82;

    return vec4(c.r * modR * vfade, c.g * modG * vfade, c.b * modB * vfade, c.a);
}