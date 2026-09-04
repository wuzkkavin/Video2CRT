//!HOOK MAIN
//!BIND HOOKED
vec4 hook(){vec4 c=HOOKED_tex(HOOKED_pos);float y=HOOKED_pos.y*HOOKED_size.y;int row=int(mod(y,3.0));float vfade=(row<1)?1.0:0.78;return vec4(c.rgb*vfade,c.a);}
