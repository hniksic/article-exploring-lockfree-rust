import java.util.concurrent.atomic.*;
import java.util.Objects;
import java.util.function.*;

public class LazyTransform<T, S> {
    private final AtomicBoolean transformLock = new AtomicBoolean();
    private final AtomicReference<S> source = new AtomicReference<>();
    private final Function<? super S, ? extends T> transformFn;
    private volatile T transformed;

    public LazyTransform(Function<? super S, ? extends T> transformFn) {
        this.transformFn = transformFn;
    }
    
    public void setSource(S src) {
        Objects.requireNonNull(src, "src");
        source.lazySet(src);
    }

    public T getTransformed() {
        if (source.get() == null || transformLock.get() || transformLock.getAndSet(true)) {
            return transformed;
        }
        try {
            final S src = source.getAndSet(null);
            if (src != null) {
                transformed = transformFn.apply(src);
            }
        } finally {
            transformLock.set(false);
        }
        return transformed;
    }
}
