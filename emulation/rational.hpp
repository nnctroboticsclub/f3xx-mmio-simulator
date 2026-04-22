#pragma once

#include <concepts>
#include <numeric>
#include <stdexcept>

namespace mcu_emulator {
template <std::integral T>
class Rational {
 public:
  Rational(T numerator, T denominator)
      : numerator_(numerator), denominator_(denominator) {
    if (denominator == 0) {
      throw std::invalid_argument("Denominator cannot be zero");
    }

    Optimize();
  }
  Rational() : Rational(1, 1) {}

  Rational operator/(T other) const {
    if (other == 0) {
      throw std::invalid_argument("Division by zero");
    }
    return Rational(numerator_, denominator_ * other);
  }

  Rational operator*(T other) const {
    return Rational(numerator_ * other, denominator_);
  }

  Rational operator*=(T other) {
    numerator_ *= other;
    Optimize();
    return *this;
  }

  Rational operator/=(T other) {
    if (other == 0) {
      throw std::invalid_argument("Division by zero");
    }
    denominator_ *= other;
    Optimize();
    return *this;
  }

  operator double() const {
    return static_cast<double>(numerator_) / static_cast<double>(denominator_);
  }

  operator std::string() const {
    return std::to_string(numerator_) + "/" + std::to_string(denominator_);
  }

  void Optimize() {
    if (denominator_ < 0) {
      numerator_ = -numerator_;
      denominator_ = -denominator_;
    }

    auto gcd = std::gcd(numerator_, denominator_);
    numerator_ /= gcd;
    denominator_ /= gcd;
  }

 private:
  T numerator_;
  T denominator_;
};
}  // namespace mcu_emulator