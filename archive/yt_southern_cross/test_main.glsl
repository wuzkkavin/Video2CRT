//!HOOK MAIN
//!BIND HOOKED
vec4 hook(){vec4 c=HOOKED_tex(HOOKED_pos);return vec4(1.0-c.rgb,c.a);}
