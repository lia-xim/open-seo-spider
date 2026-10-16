import java.lang.reflect.Method;
import java.lang.reflect.Constructor;
import java.util.Set;
import org.languagetool.server.HTTPServer;
import org.languagetool.server.HTTPServerConfig;

/** Own local probe adapter. Exact pinned external engine; stdin EOF owns shutdown. */
public class OpenSeoOwnedLanguageTool {
  public static void main(String[] args) throws Exception {
    if (args.length != 2) throw new IllegalArgumentException("port and own config path required");
    int port = Integer.parseInt(args[0]);
    if (port < 1024 || port > 65535 || port == 8787 || port == 8790)
      throw new IllegalArgumentException("invalid owned ephemeral port");
    Constructor<HTTPServerConfig> constructor = HTTPServerConfig.class.getDeclaredConstructor(String[].class);
    constructor.setAccessible(true);
    HTTPServerConfig config = constructor.newInstance((Object) new String[] {"--port", args[0], "--config", args[1]});
    // The pinned open source engine's CLI invokes this package-local bootstrap.
    // Use that same initialization for the public embeddable server, without a DB configuration.
    Class<?> bootstrap = Class.forName("org.languagetool.server.DatabaseAccess");
    Method initialize = bootstrap.getDeclaredMethod("init", HTTPServerConfig.class);
    initialize.setAccessible(true);
    initialize.invoke(null, config);
    HTTPServer engine = new HTTPServer(config, false, "127.0.0.1", Set.of("127.0.0.1"));
    try {
      engine.run();
      System.out.println("OPEN_SEO_OWNED_LANGUAGE_ENGINE_READY " + port);
      System.out.flush();
      while (System.in.read() != -1) { /* Only parent EOF requests normal shutdown. */ }
    } finally {
      engine.stop();
      System.out.println("OPEN_SEO_OWNED_LANGUAGE_ENGINE_STOPPED");
      System.out.flush();
    }
  }
}
