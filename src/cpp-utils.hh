#include <memory>

// Constructs a C++ object using this trick:
// https://github.com/dtolnay/cxx/issues/280#issuecomment-1344153115

namespace orcxx_rs {
    template<typename T, typename... Args>
    std::unique_ptr<T>
    construct(Args... args)
    {
      return std::make_unique<T>(args...);
    }
}
