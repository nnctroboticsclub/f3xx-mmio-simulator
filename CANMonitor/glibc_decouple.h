#pragma once

#undef _FORTIFY_SOURCE
#define _FORTIFY_SOURCE 0

#ifdef __cplusplus
extern "C" {
#endif

// Prevent glibc fortification macros from taking effect
#define __THROW
#define __wur
#define __attribute_pure__
#define __nonnull(x)
#define __attribute_malloc__
#define __attribute_alloc_size__(x)
#define __attribute_alloc_align__(x)
#define __attribute_warn_unused_result__
#define __attr_dealloc(x,y)
#define __fortified_attr_access(a,b,c)
#define __attr_access(x)
#define __attr_dealloc_free

#ifdef __cplusplus
}
#endif
