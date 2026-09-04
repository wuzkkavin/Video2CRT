//!HOOK MAIN
//!BIND HOOKED
vec4 hook(){vec4 c=HOOKED_tex(HOOKED_pos);float x=HOOKED_pos.x*HOOKED_size.x;int col=int(mod(x,6.0));float strength=0.13;vec3 tint=vec3(0.0);if(col<2)tint=vec3(strength,-strength*0.5,-strength*0.5);else if(col<4)tint=vec3(-strength*0.5,strength,-strength*0.5);else tint=vec3(-strength*0.5,-strength*0.5,strength);float y=HOOKED_pos.y*HOOKED_size.y;int row=int(mod(y,6.0));float vfade=(row<1)?1.0:0.7;return vec4((c.rgb+tint)*vfade,c.a);}
