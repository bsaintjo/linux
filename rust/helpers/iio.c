#include <linux/iio/iio.h>

__rust_helper void *rust_helper_iio_priv(const struct iio_dev *indio_dev)
{
	return iio_priv(indio_dev);
}