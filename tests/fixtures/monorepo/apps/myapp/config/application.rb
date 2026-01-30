# frozen_string_literal: true

require_relative "boot"
require "rails/all"

module MyApp
  class Application < Rails::Application
    config.load_defaults 8.0
  end
end
