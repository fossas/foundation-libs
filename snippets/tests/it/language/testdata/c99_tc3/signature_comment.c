/*
 * libssh2_comp_method_zlib_comp
 *
 * Compresses source to destination. Without allocation.
 */
static int
comp_method_zlib_comp(LIBSSH2_SESSION *session,
                      unsigned char *dest,

                      /* dest_len is a pointer to allow this function to
                         update it with the final actual size used */
                      size_t *dest_len,
                      const unsigned char *src,
                      size_t src_len,
                      void **abstract)
{
    z_stream *strm = *abstract;
    int out_maxlen = *dest_len;
    int status;

    strm->next_in = (unsigned char *) src;
    strm->avail_in = src_len;
    strm->next_out = dest;
    strm->avail_out = out_maxlen;

    status = deflate(strm, Z_PARTIAL_FLUSH);

    if((status == Z_OK) && (strm->avail_out > 0)) {
        *dest_len = out_maxlen - strm->avail_out;
        return 0;
    }

    _libssh2_debug(session, LIBSSH2_TRACE_TRANS,
                   "unhandled zlib compression error %d, avail_out",
                   status, strm->avail_out);
    return _libssh2_error(session, LIBSSH2_ERROR_ZLIB, "compression failure");
}
