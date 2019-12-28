import java.util.*;
import java.util.stream.*;
import java.util.concurrent.atomic.*;
import java.util.concurrent.*;
import java.util.function.*;

public class Benchmark {
    static final LazyTransform<String, byte[]> LT
        = new LazyTransform<String, byte[]>(Benchmark::blobToString);
    static volatile double BLACK_HOLE;

    public static void main(String[] args) throws Exception {
        for (int i = 0; i < 3; i++) {
            System.out.println("Start consumers");
            Thread[] consumers = new Thread[8];
            Arrays.setAll(consumers, x -> new Thread(Benchmark::consume));
            Stream.of(consumers).forEach(Thread::start);
            System.out.println("Start producing");
            produce();
            for (Thread consumer : consumers) {
                consumer.join();
            }
        }
    }

    private static void produce() {
        long start = System.nanoTime();
        final long iters = 400_000;
        for (int i = 0; i < iters; i++) {
            LT.setSource(new byte[]{randomByte(), randomByte(), randomByte()});
            simulateWork();
        }
        long elapsed = System.nanoTime() - start;
        System.out.format("Producer took %.2f ns/op (%.2f s)%n",
                          (double) elapsed / iters, elapsed / 1e9);
    }

    private static byte randomByte() {
        return (byte) ('A' + ThreadLocalRandom.current().nextInt(10));
    }

    private static void consume() {
        final long iters = 1_000_000_000;
        long start = System.nanoTime();
        long count = 0;
        for (int j = 0; j < iters; j++) {
            if ("longer".equals(LT.getTransformed())) {
                count++;
            }
        }
        long elapsed = System.nanoTime() - start;
        System.out.format("Consumer took %.2f ns/op (%.2f s, count %d)%n",
                          (double) elapsed / iters, elapsed / 1e9,
                          count);
    }

    private static void simulateWork() {
        double start = System.nanoTime();
        double d = System.nanoTime() / start;
        for (int i = 0; i < 10_000; i++) {
            d *= 1.00001;
        }
        BLACK_HOLE = d > 1.1 ? d : 2*d;
    }

    private static String blobToString(byte[] blob) {
        double start = System.nanoTime();
        double d = System.nanoTime() / start;
        for (int i = 0; i < 10_000; i++) {
            d *= 1.00001;
        }
        return new String(blob) + d;
    }
}
